# for local development - start the shared service db
surreal start --log debug --user root --pass root --bind 0.0.0.0:8001 surrealkv://./services/shared-service/db-file

# for local development - start the shared service
cargo watch -x run --workdir services/shared-service
