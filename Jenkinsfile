pipeline {
    agent {
        docker {
            image 'rust:latest'
            args '-v cargo-cache:/usr/local/cargo/registry'
        }
    }
    
    environment {
        CARGO_HOME = '/usr/local/cargo'
        RUSTUP_HOME = '/usr/local/rustup'
        PATH = "$CARGO_HOME/bin:$PATH"
        RUST_BACKTRACE = '1'
        RUST_LOG = 'debug'
    }
    
    options {
        timeout(time: 1, unit: 'HOURS')
        disableConcurrentBuilds()
    }
    
    stages {
        stage('Setup Rust') {
            steps {
                sh '''#!/bin/bash
                    # Switch to nightly
                    rustup default nightly
                    
                    # Install components
                    rustup component add rustfmt clippy rust-src
                '''
            }
        }
        
        stage('Check Format') {
            steps {
                sh 'cargo fmt --all -- --check'
            }
        }
        
        stage('Lint') {
            steps {
                sh 'cargo clippy --workspace -- -D warnings'
            }
        }
        
        stage('Build') {
            steps {
                sh 'cargo build --workspace'
            }
        }
        
        stage('Test') {
            steps {
                sh 'cargo test --workspace'
            }
        }
        
        stage('Documentation') {
            steps {
                sh 'cargo doc --workspace --no-deps'
            }
        }
        
        stage('Build Release') {
            when {
                branch 'main'
            }
            steps {
                sh 'cargo build --workspace --release'
            }
        }
    }
    
    post {
        always {
            cleanWs()
        }
        success {
            echo 'Build succeeded!'
        }
        failure {
            echo 'Build failed!'
        }
    }
}