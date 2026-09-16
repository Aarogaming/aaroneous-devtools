# Aaroneous Federation - Terraform Infrastructure as Code
# 
# Provisions complete Aaroneous Federation infrastructure on cloud platforms
# Supports: AWS, GCP, Azure with multi-region, multi-hive configurations

terraform {
  required_version = ">= 1.0"
  
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # Backend configuration for remote state
  backend "s3" {
    bucket         = "aaroneous-terraform-state"
    key            = "federation/terraform.tfstate"
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-locks"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Project     = "aaroneous-federation"
      Environment = var.environment
      ManagedBy   = "terraform"
      CreatedAt   = timestamp()
    }
  }
}

# VPC for Aaroneous infrastructure
module "vpc" {
  source = "./modules/vpc"

  name                = var.cluster_name
  cidr_block          = var.vpc_cidr
  availability_zones  = var.availability_zones
  private_subnets    = var.private_subnet_cidrs
  public_subnets     = var.public_subnet_cidrs

  enable_nat_gateway = true
  enable_vpn_gateway = var.enable_vpn

  tags = {
    Name = "${var.cluster_name}-vpc"
  }
}

# EKS Kubernetes Cluster
module "eks_cluster" {
  source = "./modules/eks"

  cluster_name    = var.cluster_name
  kubernetes_version = var.kubernetes_version
  
  subnet_ids = concat(
    module.vpc.private_subnet_ids,
    module.vpc.public_subnet_ids
  )

  enabled_cluster_log_types = [
    "api",
    "audit",
    "authenticator",
    "controllerManager",
    "scheduler"
  ]

  tags = {
    Name = "${var.cluster_name}-eks"
  }
}

# Node groups for different workload types
module "node_groups" {
  source = "./modules/node_groups"

  cluster_name       = module.eks_cluster.cluster_name
  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnet_ids

  # Specialist nodes (GPU for compute-intensive)
  specialist_node_group = {
    name           = "specialist-nodes"
    instance_types = var.specialist_instance_types
    desired_size   = var.specialist_node_count
    min_size       = var.specialist_min_nodes
    max_size       = var.specialist_max_nodes
    disk_size      = 100  # GB
    
    labels = {
      workload = "specialist"
      gpu      = "true"
    }

    taints = [
      {
        key    = "specialist"
        value  = "true"
        effect = "NoSchedule"
      }
    ]
  }

  # Sentinel nodes (orchestration)
  sentinel_node_group = {
    name           = "sentinel-nodes"
    instance_types = var.sentinel_instance_types
    desired_size   = var.sentinel_node_count
    min_size       = var.sentinel_min_nodes
    max_size       = var.sentinel_max_nodes
    disk_size      = 50  # GB

    labels = {
      workload = "sentinel"
      cpu      = "true"
    }
  }

  # System nodes (control plane, monitoring)
  system_node_group = {
    name           = "system-nodes"
    instance_types = var.system_instance_types
    desired_size   = var.system_node_count
    min_size       = 3
    max_size       = var.system_max_nodes
    disk_size      = 50  # GB

    labels = {
      workload = "system"
    }

    taints = [
      {
        key    = "system"
        value  = "true"
        effect = "NoExecute"
      }
    ]
  }
}

# RDS for DNA Bank (persistent storage)
module "rds_database" {
  source = "./modules/rds"

  identifier            = "${var.cluster_name}-dna-bank"
  engine                = "postgres"
  engine_version        = var.postgres_version
  instance_class        = var.rds_instance_type
  allocated_storage     = var.rds_storage_gb
  storage_encrypted     = true
  multi_az              = var.rds_multi_az

  db_name  = "aaroneous_federation"
  username = var.db_username
  password = var.db_password  # Use AWS Secrets Manager in production

  db_subnet_group_name   = module.vpc.db_subnet_group_id
  vpc_security_group_ids = [module.vpc.db_security_group_id]

  backup_retention_period = var.backup_retention_days
  backup_window          = "03:00-04:00"
  maintenance_window     = "sun:04:00-sun:05:00"

  enable_iam_database_authentication = true

  skip_final_snapshot = !var.create_final_snapshot
  final_snapshot_identifier = "${var.cluster_name}-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"

  tags = {
    Name = "${var.cluster_name}-dna-bank"
  }
}

# ElastiCache Redis for audit log caching
module "redis_cache" {
  source = "./modules/elasticache"

  identifier           = "${var.cluster_name}-audit-cache"
  engine               = "redis"
  engine_version       = var.redis_version
  node_type            = var.redis_node_type
  num_cache_nodes      = var.redis_num_nodes
  parameter_group_name = "default.redis7"
  port                 = 6379

  subnet_group_name = module.vpc.elasticache_subnet_group_id
  security_group_ids = [module.vpc.redis_security_group_id]

  automatic_failover_enabled = var.redis_multi_az
  multi_az_enabled          = var.redis_multi_az
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true

  snapshot_retention_limit = 7
  snapshot_window          = "03:00-05:00"

  log_delivery_configuration = {
    slow-log = {
      cloudwatch_log_group_name = aws_cloudwatch_log_group.redis_slow_log.name
      log_format                = "json"
      enabled                   = true
    }
  }

  tags = {
    Name = "${var.cluster_name}-audit-cache"
  }
}

# S3 for model storage and backups
module "s3_storage" {
  source = "./modules/s3"

  bucket_name = "${var.cluster_name}-storage-${data.aws_caller_identity.current.account_id}"
  
  versioning = {
    enabled = true
  }

  server_side_encryption_configuration = {
    rule = {
      apply_server_side_encryption_by_default = {
        sse_algorithm = "AES256"
      }
    }
  }

  lifecycle_rule = [
    {
      id     = "archive-old-models"
      status = "Enabled"

      noncurrent_version_transition = {
        days          = 30
        storage_class = "GLACIER"
      }

      noncurrent_version_expiration = {
        days = 90
      }
    }
  ]

  acl = "private"
  block_public_acls = true
  block_public_policy = true
  ignore_public_acls = true
  restrict_public_buckets = true

  tags = {
    Name = "${var.cluster_name}-storage"
  }
}

# CloudWatch Log Groups
resource "aws_cloudwatch_log_group" "aaroneous" {
  name              = "/aws/aaroneous/${var.cluster_name}"
  retention_in_days = var.log_retention_days

  tags = {
    Name = "${var.cluster_name}-logs"
  }
}

resource "aws_cloudwatch_log_group" "redis_slow_log" {
  name              = "/aws/elasticache/${var.cluster_name}/slow-log"
  retention_in_days = var.log_retention_days

  tags = {
    Name = "${var.cluster_name}-redis-logs"
  }
}

# Outputs
output "eks_cluster_name" {
  description = "Name of the EKS cluster"
  value       = module.eks_cluster.cluster_name
}

output "eks_cluster_endpoint" {
  description = "Endpoint for the EKS cluster"
  value       = module.eks_cluster.cluster_endpoint
  sensitive   = true
}

output "rds_endpoint" {
  description = "RDS database endpoint"
  value       = module.rds_database.endpoint
}

output "redis_endpoint" {
  description = "Redis cache endpoint"
  value       = module.redis_cache.primary_endpoint_address
}

output "s3_bucket_name" {
  description = "S3 bucket for model storage"
  value       = module.s3_storage.bucket_name
}

output "configure_kubectl" {
  description = "Command to configure kubectl"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${module.eks_cluster.cluster_name}"
}

# Data source for current AWS account
data "aws_caller_identity" "current" {}
