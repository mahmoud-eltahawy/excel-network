

# Test Dependencies
1. [**Rust Programming Language**](https://www.rust-lang.org/)
2. [**tauri prerequisites**](https://tauri.app/v1/guides/getting-started/prerequisites)
3. ### **cargo-make**
   cargo install cargo-make
4. [**docker**](https://www.docker.com/)


# How to Test
## clone the repo
   git clone https://github.com/mahmoud-eltahawy/excel-network.git
## create postgres docker image 
   docker compose -f postgres-docker.yml up
## run the api
   cargo make api
## run the ui
   cargo make ui


# How to build
   cargo make build
