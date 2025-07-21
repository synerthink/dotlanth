#!/usr/bin/env sh

echo "=== Dotlanth FreeBSD Deployment Automation ==="

# Check if running as root
if [ "$(id -u)" != "0" ]; then
    echo "❌ ERROR: This command must be run as root (use sudo)"
    echo "Usage: sudo ./scripts/deploy-freebsd.sh"
    exit 1
fi

echo "✅ Running as root"

# Step 1: Check built binaries
echo "\n=== Step 1: Checking built binaries ==="
if [ ! -d "target/release" ]; then
    echo "❌ ERROR: No release build found. Run 'just build-release' first"
    exit 1
fi

echo "Built binaries:"
ls -la target/release/ | grep "^-rwx" | awk '{print "  " $9}'

# Step 2: Create dotlanth user
echo "\n=== Step 2: Creating dotlanth user ==="
if id dotlanth >/dev/null 2>&1; then
    echo "✅ User 'dotlanth' already exists"
else
    echo "Creating user 'dotlanth'..."
    pw useradd dotlanth -d /var/lib/dotlanth -s /usr/sbin/nologin
    echo "✅ User 'dotlanth' created"
fi

# Step 3: Create directories
echo "\n=== Step 3: Creating directories ==="
mkdir -p /var/lib/dotlanth
mkdir -p /usr/local/etc/dotlanth
mkdir -p /var/log/dotlanth
mkdir -p /zroot/dotlanth/data
mkdir -p /zroot/dotlanth/logs
echo "✅ All directories created"

# Step 4: Set ownership
echo "\n=== Step 4: Setting ownership ==="
chown -R dotlanth:dotlanth /var/lib/dotlanth
chown -R dotlanth:dotlanth /var/log/dotlanth
chown -R dotlanth:dotlanth /zroot/dotlanth
echo "✅ Ownership set"

# Step 5: Install binaries
echo "\n=== Step 5: Installing binaries ==="
for binary in dotvm dotvm-runtime dotlanth dotdb; do
    if [ -f "target/release/$binary" ]; then
        cp "target/release/$binary" /usr/local/bin/
        chmod +x "/usr/local/bin/$binary"
        echo "✅ Installed: $binary"
    else
        echo "⚠️  Skipped: $binary (not found)"
    fi
done

# Step 6: Detect correct server binary
echo "\n=== Step 6: Detecting server binary ==="
SERVER_BINARY=""
SERVER_ARGS=""

# Test dotvm-runtime first (most likely to be the server)
if [ -f "/usr/local/bin/dotvm-runtime" ]; then
    echo "Testing dotvm-runtime..."
    if /usr/local/bin/dotvm-runtime --help >/dev/null 2>&1; then
        SERVER_BINARY="/usr/local/bin/dotvm-runtime"
        echo "✅ Using dotvm-runtime as server"
    fi
fi

# Test dotlanth if dotvm-runtime didn't work
if [ -z "$SERVER_BINARY" ] && [ -f "/usr/local/bin/dotlanth" ]; then
    echo "Testing dotlanth..."
    if /usr/local/bin/dotlanth --help >/dev/null 2>&1; then
        SERVER_BINARY="/usr/local/bin/dotlanth"
        echo "✅ Using dotlanth as server"
    fi
fi

# Test dotvm with subcommands
if [ -z "$SERVER_BINARY" ] && [ -f "/usr/local/bin/dotvm" ]; then
    echo "Testing dotvm subcommands..."
    for cmd in server run start; do
        if /usr/local/bin/dotvm $cmd --help >/dev/null 2>&1; then
            SERVER_BINARY="/usr/local/bin/dotvm"
            SERVER_ARGS="$cmd"
            echo "✅ Using dotvm $cmd as server"
            break
        fi
    done
fi

# Fallback to dotvm without args
if [ -z "$SERVER_BINARY" ]; then
    SERVER_BINARY="/usr/local/bin/dotvm"
    echo "⚠️  Using dotvm as fallback (may need manual configuration)"
fi

# Step 7: Create config file
echo "\n=== Step 7: Creating config file ==="
cat > /usr/local/etc/dotlanth/config.toml << 'EOF'
[server]
host = "0.0.0.0"
port = 8080
grpc_port = 50051

[database]
path = "/zroot/dotlanth/data"

[logging]
level = "info"
path = "/zroot/dotlanth/logs"
EOF
chown dotlanth:dotlanth /usr/local/etc/dotlanth/config.toml
echo "✅ Config file created"

# Step 8: Create service script
echo "\n=== Step 8: Creating service script ==="
cat > /usr/local/etc/rc.d/dotlanth << EOF
#!/bin/sh
# PROVIDE: dotlanth
# REQUIRE: NETWORKING
# KEYWORD: shutdown

. /etc/rc.subr

name="dotlanth"
rcvar="dotlanth_enable"
command="$SERVER_BINARY"
command_args="$SERVER_ARGS"
pidfile="/var/run/dotlanth.pid"
dotlanth_user="dotlanth"
dotlanth_group="dotlanth"

start_precmd="dotlanth_prestart"
dotlanth_prestart() {
    if [ ! -d "/var/lib/dotlanth" ]; then
        mkdir -p /var/lib/dotlanth
        chown dotlanth:dotlanth /var/lib/dotlanth
    fi
    if [ ! -d "/zroot/dotlanth/data" ]; then
        mkdir -p /zroot/dotlanth/data
        chown dotlanth:dotlanth /zroot/dotlanth/data
    fi
    if [ ! -d "/zroot/dotlanth/logs" ]; then
        mkdir -p /zroot/dotlanth/logs
        chown dotlanth:dotlanth /zroot/dotlanth/logs
    fi
}

load_rc_config \$name
run_rc_command "\$1"
EOF
chmod +x /usr/local/etc/rc.d/dotlanth
echo "✅ Service script created with: $SERVER_BINARY $SERVER_ARGS"

# Step 9: Enable service
echo "\n=== Step 9: Enabling service ==="
sysrc dotlanth_enable=YES
echo "✅ Service enabled"

echo "\n=== Deployment Complete! ==="
echo "Next steps:"
echo "  1. Start service: just freebsd-start"
echo "  2. Setup monitoring: just freebsd-monitoring"
echo "  3. Check status: just freebsd-status"
echo "  4. Test service: just freebsd-test"