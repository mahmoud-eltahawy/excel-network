

# Test Dependencies
1. (**Rust Programming Language**)[https://www.rust-lang.org/]
2. (**tauri prerequisites**)[https://tauri.app/v1/guides/getting-started/prerequisites]
3. **cargo-make** __cargo install cargo-make__
4. (**docker**)[https://www.docker.com/]


# How to Test
1. clone the repo
2. docker compose -f postgres-docker.yml up
3. cargo make api
4. cargo make ui
