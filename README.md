# vivi Rust Template

A template for a Rust application that's using [Slint](https://slint.rs) for the user interface and `vivi` as component library.

## Usage

1. Install Rust by following the [Rust Getting Started Guide](https://www.rust-lang.org/learn/get-started).
   Once this is done, you should have the ```rustc``` compiler and the ```cargo``` build system installed in your path.
2. Install [`cargo-generate`](https://github.com/cargo-generate/cargo-generate)
    ```
    cargo install cargo-generate
    ```
3. Set up a sample project with this template
    ```
    cargo generate --git https://ash.radicle.garden/z3xAtPtoBbxAza3RRJDqaNHVzju9v.git --name my-project
    cd my-project
    ```
3. Build with cargo
    ```
    cargo build
    ```
4. Run the application binary
     ```
     cargo run
     ```