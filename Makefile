.PHONY: help build up down logs restart clean ps health deploy-example test-network

# Default target
help:
	@echo "Pied Piper Docker Commands"
	@echo "=========================="
	@echo ""
	@echo "  make build          - Build Docker images"
	@echo "  make up             - Start all nodes"
	@echo "  make down           - Stop all nodes"
	@echo "  make restart        - Restart all nodes"
	@echo "  make logs           - View logs from all nodes"
	@echo "  make ps             - Show running containers"
	@echo "  make health         - Check health status"
	@echo "  make clean          - Stop and remove all data"
	@echo "  make deploy-example - Deploy hello-api example"
	@echo "  make test-network   - Test network connectivity"
	@echo ""
	@echo "Node-specific commands:"
	@echo "  make logs-bootstrap - View bootstrap node logs"
	@echo "  make logs-node1     - View node 1 logs"
	@echo "  make logs-node2     - View node 2 logs"
	@echo "  make logs-node3     - View node 3 logs"
	@echo ""
	@echo "  make shell-bootstrap - Shell into bootstrap node"
	@echo "  make shell-node1     - Shell into node 1"
	@echo "  make shell-node2     - Shell into node 2"
	@echo "  make shell-node3     - Shell into node 3"

# Build Docker images
build:
	docker compose build

# Start all nodes
up:
	docker compose up -d
	@echo "Waiting for nodes to start..."
	@sleep 5
	@make health

# Stop all nodes
down:
	docker compose down

# Restart all nodes
restart: down up

# View logs from all nodes
logs:
	docker compose logs -f

# View logs from specific nodes
logs-bootstrap:
	docker compose logs -f node-bootstrap

logs-node1:
	docker compose logs -f node-1

logs-node2:
	docker compose logs -f node-2

logs-node3:
	docker compose logs -f node-3

# Show running containers
ps:
	docker compose ps

# Check health status
health:
	@echo "Checking node health..."
	@docker compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Ports}}"
	@echo ""
	@echo "Bootstrap Node: http://localhost:8080"
	@curl -s http://localhost:8080/health > /dev/null && echo "  ✓ Healthy" || echo "  ✗ Unhealthy"
	@echo "Node 1: http://localhost:8081"
	@curl -s http://localhost:8081/health > /dev/null && echo "  ✓ Healthy" || echo "  ✗ Unhealthy"
	@echo "Node 2: http://localhost:8082"
	@curl -s http://localhost:8082/health > /dev/null && echo "  ✓ Healthy" || echo "  ✗ Unhealthy"
	@echo "Node 3: http://localhost:8083"
	@curl -s http://localhost:8083/health > /dev/null && echo "  ✓ Healthy" || echo "  ✗ Unhealthy"

# Stop and remove all data
clean:
	docker compose down -v
	docker system prune -f

# Shell into nodes
shell-bootstrap:
	docker exec -it pied-piper-bootstrap /bin/bash

shell-node1:
	docker exec -it pied-piper-node-1 /bin/bash

shell-node2:
	docker exec -it pied-piper-node-2 /bin/bash

shell-node3:
	docker exec -it pied-piper-node-3 /bin/bash

# Deploy hello-api example (requires local build)
deploy-example:
	@echo "Building hello-api example..."
	@cd examples/wasip1-core/hello-api && \
		cargo build --target wasm32-wasip1 --release
	@echo "Deploying to bootstrap node..."
	@./target/release/pied-piper deploy \
		--gateway http://localhost:8080 \
		--file examples/wasip1-core/hello-api/target/wasm32-wasip1/release/hello_api.wasm
	@echo ""
	@echo "Example deployed! Test with:"
	@echo "  curl http://localhost:8080/cid/<CID>"

# Test network connectivity
test-network:
	@echo "Testing network connectivity..."
	@echo ""
	@echo "Bootstrap → Node 1:"
	@docker exec pied-piper-bootstrap ping -c 2 172.20.0.11
	@echo ""
	@echo "Node 1 → Node 2:"
	@docker exec pied-piper-node-1 ping -c 2 172.20.0.12
	@echo ""
	@echo "Node 2 → Node 3:"
	@docker exec pied-piper-node-2 ping -c 2 172.20.0.13
	@echo ""
	@echo "Network connectivity OK ✓"

# Monitor resource usage
stats:
	docker stats --no-stream

# Backup all node data
backup:
	@mkdir -p backups
	@echo "Backing up node data..."
	@docker run --rm \
		-v pied-piper_bootstrap-data:/data \
		-v $(PWD)/backups:/backup \
		debian:bookworm-slim \
		tar czf /backup/bootstrap-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	@docker run --rm \
		-v pied-piper_node-1-data:/data \
		-v $(PWD)/backups:/backup \
		debian:bookworm-slim \
		tar czf /backup/node-1-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	@docker run --rm \
		-v pied-piper_node-2-data:/data \
		-v $(PWD)/backups:/backup \
		debian:bookworm-slim \
		tar czf /backup/node-2-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	@docker run --rm \
		-v pied-piper_node-3-data:/data \
		-v $(PWD)/backups:/backup \
		debian:bookworm-slim \
		tar czf /backup/node-3-$(shell date +%Y%m%d-%H%M%S).tar.gz -C /data .
	@echo "Backup complete! Files in ./backups/"

# Show disk usage
disk-usage:
	@echo "Docker disk usage:"
	@docker system df
	@echo ""
	@echo "Volume sizes:"
	@docker system df -v | grep pied-piper
