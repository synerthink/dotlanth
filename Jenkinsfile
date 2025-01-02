pipeline {
    agent any
    
    environment {
        CARGO_HOME = '/var/jenkins_home/.cargo'
        RUSTUP_HOME = '/var/jenkins_home/.rustup'
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
                    # Install rustup
                    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
                    
                    # Load the cargo environment
                    . "$CARGO_HOME/env"
                    
                    # Install components
                    rustup component add rustfmt clippy rust-src
                '''
            }
        }
        
        stage('Check Format') {
            steps {
                sh '''#!/bin/bash
                    . "$CARGO_HOME/env"
                    cargo fmt --all -- --check
                '''
            }
        }
        
        stage('Lint') {
            steps {
                sh '''#!/bin/bash
                    . "$CARGO_HOME/env"
                    cargo clippy --workspace -- -D warnings
                '''
            }
        }
        
        stage('Build') {
            steps {
                sh '''#!/bin/bash
                    . "$CARGO_HOME/env"
                    cargo build --workspace
                '''
            }
        }
        
        stage('Test') {
            steps {
                sh '''#!/bin/bash
                    . "$CARGO_HOME/env"
                    cargo test --workspace
                '''
            }
        }
        
        stage('Documentation') {
            steps {
                sh '''#!/bin/bash
                    . "$CARGO_HOME/env"
                    cargo doc --workspace --no-deps
                '''
            }
        }
        
        stage('Build Release') {
            when {
                branch 'main'
            }
            steps {
                sh '''#!/bin/bash
                    . "$CARGO_HOME/env"
                    cargo build --workspace --release
                '''
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