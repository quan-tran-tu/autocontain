# autocontain
A simple project to automate trying out github repos.

## Requirements:
- **Rust**: Install Rust if you plan to build and run the project directly. You can install Rust using [Rustup](https://rustup.rs/):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
- **Docker**: Currently not needed to test out the project.
## Setup
1. Clone the repository:
```bash
git clone https://github.com/quan-tran-tu/autocontain.git
cd autocontain
```
## Usage:
### Example: 
```bash
cargo run -- https://github.com/drawdb-io/drawdb [--persist] [--depth]
```
### Params:
- --persist: save the repo and the content generated.
- --depth: specify how deep the project should search for markdown files.