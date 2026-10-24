#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"
.venv/bin/python -m pip install --require-hashes -r .devcontainer/requirements-jupyter.txt
.venv/bin/python -m ipykernel install --user --name rvt-rs --display-name 'rvt-rs (Python 3.12)'
mkdir -p "$HOME/.local/bin"
for command in jupyter jupyter-lab; do
  destination="$HOME/.local/bin/$command"
  if [[ -e "$destination" || -L "$destination" ]]; then
    [[ "$(readlink "$destination")" = "$root/.venv/bin/$command" ]] || {
      echo "Preserving an existing notebook command: $destination" >&2
      exit 1
    }
  else
    ln -s "$root/.venv/bin/$command" "$destination"
  fi
done
.venv/bin/jupyter lab --version
.venv/bin/jupyter kernelspec list
