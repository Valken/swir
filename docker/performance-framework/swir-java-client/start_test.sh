curl -v -d '{"messages":1000, "threads":2, "sidecarUrl":"http://127.0.0.1:8080/publish"}' -H "Content-Type: application/json" -X POST http://localhost:8090/test