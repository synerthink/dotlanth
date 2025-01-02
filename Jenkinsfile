pipeline {
    agent any
    
    stages {
        stage('Build and Test') {
            agent {
                docker {
                    image 'rust:latest'
                    args '-v cargo-cache:/usr/local/cargo/registry --user root'
                }
            }
            stages {
                stage('Setup Rust') {
                    steps {
                        sh '''#!/bin/bash
                            # Ensure directory permissions
                            mkdir -p /usr/local/cargo/registry
                            chmod -R 777 /usr/local/cargo/registry
                            
                            rustup default nightly
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