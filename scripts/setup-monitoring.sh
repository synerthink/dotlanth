#!/usr/bin/env sh

echo "=== Setting up Monitoring Stack ==="

# Check if running as root
if [ "$(id -u)" != "0" ]; then
    echo "❌ ERROR: This command must be run as root (use sudo)"
    echo "Usage: sudo ./scripts/setup-monitoring.sh"
    exit 1
fi

# Install monitoring packages
echo "Installing monitoring packages..."
pkg install -y prometheus grafana9 node_exporter
echo "✅ Monitoring packages installed"

# Create Prometheus config
echo "Creating Prometheus configuration..."
mkdir -p /usr/local/etc/prometheus
cat > /usr/local/etc/prometheus/prometheus.yml << 'EOF'
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'prometheus'
    static_configs:
      - targets: ['localhost:9090']

  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']

  - job_name: 'dotlanth'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
    scrape_interval: 5s

  - job_name: 'dotlanth-grpc'
    static_configs:
      - targets: ['localhost:50051']
    metrics_path: '/metrics'
    scrape_interval: 5s
EOF
echo "✅ Prometheus config created"

# Enable and start services
echo "Enabling monitoring services..."
sysrc prometheus_enable=YES
sysrc grafana_enable=YES
sysrc node_exporter_enable=YES

echo "Starting monitoring services..."
service prometheus start
service grafana start
service node_exporter start

echo "\n=== Monitoring Setup Complete! ==="
echo "Access points:"
echo "  - Prometheus: http://localhost:9090"
echo "  - Grafana: http://localhost:3000 (admin/admin)"
echo "  - Node Exporter: http://localhost:9100/metrics"