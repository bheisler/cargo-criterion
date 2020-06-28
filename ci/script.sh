set -ex

if [ "$CLIPPY" = "yes" ]; then
      cargo clippy --all -- -D warnings
elif [ "$DOCS" = "yes" ]; then
    cargo clean
    cargo doc --all --no-deps
    cd book
    mdbook build
    cd ..
    cp -r book/book/html/ target/doc/book/
    travis-cargo doc-upload || true
elif [ "$RUSTFMT" = "yes" ]; then
    cargo fmt --all -- --check
else
    cargo check --no-default-features
    cargo check --no-default-features --features gnuplot_backend
    cargo check --no-default-features --features plotters_backend

    cargo check --all-features

    cargo test   
fi
