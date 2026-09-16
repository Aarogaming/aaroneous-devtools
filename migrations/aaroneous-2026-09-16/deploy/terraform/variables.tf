# Terraform Variables for Aaroneous Federation Deployment

# AWS Configuration
variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name (dev, staging, production)"
  type        = string
  default     = "production"
  
  validation {
    condition     = contains(["dev", "staging", "production"], var.environment)
    error_message = "Environment must be dev, staging, or production."
  }
}

# Cluster Configuration
variable "cluster_name" {
  description = "Name of the Aaroneous Federation cluster"
  type        = string
  default     = "aaroneous-federation"
}

variable "kubernetes_version" {
  description = "Kubernetes version"
  type        = string
  default     = "1.27"
}

# VPC Configuration
variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones" {
  description = "Availability zones"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b", "us-east-1c"]
}

variable "public_subnet_cidrs" {
  description = "CIDR blocks for public subnets"
  type        = list(string)
  default     = ["10.0.1.0/24", "10.0.2.0/24", "10.0.3.0/24"]
}

variable "private_subnet_cidrs" {
  description = "CIDR blocks for private subnets"
  type        = list(string)
  default     = ["10.0.11.0/24", "10.0.12.0/24", "10.0.13.0/24"]
}

variable "enable_vpn" {
  description = "Enable VPN gateway"
  type        = bool
  default     = false
}

# Node Group Configuration
variable "specialist_instance_types" {
  description = "EC2 instance types for specialist nodes (GPU)"
  type        = list(string)
  default     = ["g4dn.xlarge"]  # GPU instance
}

variable "specialist_node_count" {
  description = "Desired number of specialist nodes"
  type        = number
  default     = 3
}

variable "specialist_min_nodes" {
  description = "Minimum specialist nodes"
  type        = number
  default     = 1
}

variable "specialist_max_nodes" {
  description = "Maximum specialist nodes"
  type        = number
  default     = 10
}

variable "sentinel_instance_types" {
  description = "EC2 instance types for sentinel nodes"
  type        = list(string)
  default     = ["t3.xlarge"]  # CPU instance
}

variable "sentinel_node_count" {
  description = "Desired number of sentinel nodes"
  type        = number
  default     = 3
}

variable "sentinel_min_nodes" {
  description = "Minimum sentinel nodes"
  type        = number
  default     = 1
}

variable "sentinel_max_nodes" {
  description = "Maximum sentinel nodes"
  type        = number
  default     = 10
}

variable "system_instance_types" {
  description = "EC2 instance types for system nodes"
  type        = list(string)
  default     = ["t3.large"]
}

variable "system_node_count" {
  description = "Desired number of system nodes"
  type        = number
  default     = 3
}

variable "system_max_nodes" {
  description = "Maximum system nodes"
  type        = number
  default     = 5
}

# RDS Configuration
variable "rds_instance_type" {
  description = "RDS instance type"
  type        = string
  default     = "db.t3.medium"
}

variable "rds_storage_gb" {
  description = "RDS allocated storage in GB"
  type        = number
  default     = 100
}

variable "rds_multi_az" {
  description = "Enable RDS Multi-AZ"
  type        = bool
  default     = true
}

variable "postgres_version" {
  description = "PostgreSQL version"
  type        = string
  default     = "15"
}

variable "db_username" {
  description = "RDS database username"
  type        = string
  sensitive   = true
  default     = "aaroneous_admin"
}

variable "db_password" {
  description = "RDS database password"
  type        = string
  sensitive   = true
}

variable "backup_retention_days" {
  description = "RDS backup retention period"
  type        = number
  default     = 30
}

# Redis Configuration
variable "redis_version" {
  description = "Redis version"
  type        = string
  default     = "7.0"
}

variable "redis_node_type" {
  description = "ElastiCache node type"
  type        = string
  default     = "cache.t3.medium"
}

variable "redis_num_nodes" {
  description = "Number of Redis nodes"
  type        = number
  default     = 3
}

variable "redis_multi_az" {
  description = "Enable Redis Multi-AZ"
  type        = bool
  default     = true
}

# Backup Configuration
variable "create_final_snapshot" {
  description = "Create final snapshot on destroy"
  type        = bool
  default     = true
}

# Logging Configuration
variable "log_retention_days" {
  description = "CloudWatch log retention in days"
  type        = number
  default     = 30
}

# Tags
variable "additional_tags" {
  description = "Additional tags to apply to resources"
  type        = map(string)
  default     = {}
}
