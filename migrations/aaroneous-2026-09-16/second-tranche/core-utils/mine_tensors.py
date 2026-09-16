import json
from pathlib import Path
from gguf import GGUFReader

def mine_repo(repo_path: Path):
    """Indices all GGUF tensors in a repository with precise byte offsets."""
    registry = {}
    for gguf_path in repo_path.glob("**/*.gguf"):
        print(f"Mining: {gguf_path.name}")
        try:
            reader = GGUFReader(str(gguf_path))
            tensors = {}
            for tensor in reader.tensors:
                tensors[tensor.name] = {
                    "shape": tensor.shape.tolist(),
                    "type": str(tensor.tensor_type),
                    "offset": tensor.data_offset,
                    "size": tensor.data.nbytes
                }
            registry[gguf_path.name] = {
                "path": str(gguf_path),
                "tensor_count": len(tensors),
                "tensors": tensors
            }
        except Exception as e:
            print(f"Failed to mine {gguf_path.name}: {e}")
    
    output_path = Path("D:\\Aaroneous\\registry\\tensor_index.json")
    with open(output_path, "w") as f:
        json.dump(registry, f, indent=2)
    print(f"Tensor Index crystallized at {output_path}")

if __name__ == "__main__":
    mine_repo(Path("D:\\AaroneousAutomationSuite"))
