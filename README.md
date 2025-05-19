# CRFS - Conflict-Free Replicated File System
Cambridge University Computer Science Tripos Part II Project

## Client (`./CRFS`)

```bash
cd CRFS

# SETUP
cargo install

# Run with...
cargo run -- <opts>

# Or install with...
cargo build -r && cp ./target/release/CRFS ~/.local/bin
# If ~/.local/bin doesn't exist, replace with a dir on PATH
# And run with...
CRFS <opts>
```

## Server (`./CRFS_Server`)

Prerequisites:
- python3, python3-pip, python3-venv

```bash
cd CRFS_Server

# Setup
python3 -m venv env
source env/bin/activate
pip install -r requirements.txt

./manage.py migrate  # Apply database migrations

# Run
./manage.py runserver 0.0.0.0:8000
```
