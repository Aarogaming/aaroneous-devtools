#!/usr/bin/env python3
"""
omni_spatial_projector.py
Aaroneous Omni Spatial Projector:
Scans codebase files/data nodes, computes 3D spatial semantic coordinates, clusters them into galaxies,
and outputs an Omni 3D Constellation Galaxy JSON dataset.
"""

import os
import sys
import json
import math
import hashlib

def compute_spatial_coordinates(file_path, content=""):
    """
    Computes (X, Y, Z) coordinates based on domain, temporal lifecycle, and priority.
    - X (Domain Spectrum): -1000.0 (Doc/Spec/Theory) to +1000.0 (Binary/Execution/HID)
    - Y (Temporal Phase): -800.0 (History/Archived) to +800.0 (Active/WIP/Future)
    - Z (Priority): -500.0 (Deep Background/Hidden) to +800.0 (Critical/Core)
    """
    path_lower = file_path.lower()

    # Determine X (Domain)
    if "doc" in path_lower or "readme" in path_lower or "manifesto" in path_lower:
        x = -600.0
    elif "spec" in path_lower or "protocol" in path_lower or "config" in path_lower:
        x = -200.0
    elif "test" in path_lower or "qa" in path_lower or "audit" in path_lower:
        x = 100.0
    elif "hid" in path_lower or "capture" in path_lower or "reflex" in path_lower or "gpu" in path_lower:
        x = 800.0
    elif "core" in path_lower or "hypervisor" in path_lower or "stem_cell" in path_lower:
        x = 500.0
    else:
        x = 0.0

    # Determine Y (Temporal / Lifecycle)
    if "history" in path_lower or "deprecated" in path_lower or "archive" in path_lower:
        y = -700.0
    elif "dev" in path_lower or "wip" in path_lower or "roadmap" in path_lower:
        y = 500.0
    elif "active" in path_lower or "components" in path_lower:
        y = 100.0
    else:
        y = 0.0

    # Determine Z (Priority)
    if "hypervisor" in path_lower or "stem_cell" in path_lower or "linking_protocol" in path_lower:
        z = 750.0
    elif "marionette" in path_lower or "chimera" in path_lower or "odin" in path_lower or "merlin" in path_lower:
        z = 600.0
    elif "tools" in path_lower or "diagnostics" in path_lower:
        z = 400.0
    else:
        z = 100.0

    # Add hash-based subtle jitter to prevent perfect overlap
    h = int(hashlib.md5(file_path.encode()).hexdigest()[:6], 16)
    jitter_x = (h % 100) - 50
    jitter_y = ((h >> 4) % 100) - 50
    jitter_z = ((h >> 8) % 100) - 50

    return x + jitter_x, y + jitter_y, z + jitter_z

def determine_node_type(file_path):
    p = file_path.lower()
    if "doc" in p or "md" in p:
        return "Lore"
    if "bug" in p or "err" in p or "fix" in p:
        return "Bug"
    if "test" in p:
        return "TestCase"
    if "spec" in p or "protocol" in p:
        return "Architecture"
    if "shader" in p or "wgsl" in p or "reflex" in p:
        return "NeuralSignal"
    return "Feature"

def cluster_nodes(nodes, threshold=250.0):
    """Group star-nodes within spatial threshold into galaxies."""
    clusters = []
    visited = set()

    for i, node_a in enumerate(nodes):
        if i in visited:
            continue
        cluster_members = [node_a]
        visited.add(i)

        for j, node_b in enumerate(nodes):
            if j in visited:
                continue
            dx = node_a["spatial_coord"]["x"] - node_b["spatial_coord"]["x"]
            dy = node_a["spatial_coord"]["y"] - node_b["spatial_coord"]["y"]
            dz = node_a["spatial_coord"]["z"] - node_b["spatial_coord"]["z"]
            dist = math.sqrt(dx*dx + dy*dy + dz*dz)

            if dist <= threshold:
                cluster_members.append(node_b)
                visited.add(j)

        if len(cluster_members) >= 2:
            center_x = sum(n["spatial_coord"]["x"] for n in cluster_members) / len(cluster_members)
            center_y = sum(n["spatial_coord"]["y"] for n in cluster_members) / len(cluster_members)
            center_z = sum(n["spatial_coord"]["z"] for n in cluster_members) / len(cluster_members)
            clusters.append({
                "galaxy_id": f"galaxy_{len(clusters) + 1}",
                "name": f"{cluster_members[0]['domain']} Galaxy",
                "center": {"x": round(center_x, 1), "y": round(center_y, 1), "z": round(center_z, 1)},
                "star_count": len(cluster_members),
                "stars": [n["id"] for n in cluster_members]
            })

    return clusters

def main():
    workspace_root = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", ".."))
    print("=" * 60)
    print("  Aaroneous Omni Spatial Galaxy Projector")
    print(f"  Scanning: {workspace_root}")
    print("=" * 60)

    nodes = []
    ignored = {"target", "node_modules", ".git", "cache", ".cargo", "dist", "fabrication"}

    for root, dirs, files in os.walk(workspace_root):
        dirs[:] = [d for d in dirs if d not in ignored]
        for f in files:
            full_path = os.path.join(root, f)
            rel_path = os.path.relpath(full_path, workspace_root)
            x, y, z = compute_spatial_coordinates(rel_path)
            node_type = determine_node_type(rel_path)
            domain = rel_path.split(os.sep)[0]

            nodes.append({
                "id": rel_path.replace("\\", "/"),
                "title": f,
                "node_type": node_type,
                "domain": domain,
                "spatial_coord": {"x": round(x, 1), "y": round(y, 1), "z": round(z, 1)},
                "activity_pulse": 0.8 if "dev" in rel_path else 0.2,
                "uri": f"file:///{full_path.replace(os.sep, '/')}"
            })

    galaxies = cluster_nodes(nodes)

    omni_galaxy_dataset = {
        "omni_version": "1.0.0",
        "total_stars": len(nodes),
        "total_galaxies": len(galaxies),
        "galaxies": galaxies,
        "star_nodes": nodes
    }

    out_dir = os.path.join(workspace_root, "dev", "tools", "omni", "output")
    os.makedirs(out_dir, exist_ok=True)
    out_file = os.path.join(out_dir, "omni_galaxy_map.json")

    with open(out_file, "w", encoding="utf-8") as f:
        json.dump(omni_galaxy_dataset, f, indent=2)

    print(f"\n[+] Omni Galaxy Projection complete!")
    print(f"    - Projected Stars: {len(nodes)}")
    print(f"    - Clustered Galaxies: {len(galaxies)}")
    print(f"    - Output Map: {out_file}")

if __name__ == "__main__":
    main()
