FROM golang:1.24 AS builder

WORKDIR /app

# Copy module files first so Docker can cache the dependency layer
COPY go.mod go.sum ./
RUN go mod download

# Copy source and generated pb code
COPY . .

# Build a statically linked binary
RUN CGO_ENABLED=0 GOOS=linux go build -o key-server .

# Final image using alpine as base
FROM alpine:3.19

WORKDIR /app
COPY --from=builder /app/key-server .

EXPOSE 50051
ENTRYPOINT ["/app/key-server"]
