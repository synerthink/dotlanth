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
                        githubCommitStatus(context: 'Setup', description: 'Setting up Rust...', status: 'PENDING')
                        sh '''#!/bin/bash
                            # Ensure directory permissions
                            mkdir -p /usr/local/cargo/registry
                            chmod -R 777 /usr/local/cargo/registry
                            rustup default nightly
                            rustup component add rustfmt clippy rust-src
                        '''
                        githubCommitStatus(context: 'Setup', description: 'Rust setup complete', status: 'SUCCESS')
                    }
                }
                stage('Check Format') {
                    steps {
                        githubCommitStatus(context: 'Format', description: 'Checking format...', status: 'PENDING')
                        sh 'cargo fmt --all -- --check'
                        githubCommitStatus(context: 'Format', description: 'Format check passed', status: 'SUCCESS')
                    }
                }
                stage('Lint') {
                    steps {
                        githubCommitStatus(context: 'Lint', description: 'Running clippy...', status: 'PENDING')
                        sh 'cargo clippy --workspace -- -D warnings'
                        githubCommitStatus(context: 'Lint', description: 'Lint passed', status: 'SUCCESS')
                    }
                }
                stage('Build') {
                    steps {
                        githubCommitStatus(context: 'Build', description: 'Building...', status: 'PENDING')
                        sh 'cargo build --workspace'
                        githubCommitStatus(context: 'Build', description: 'Build successful', status: 'SUCCESS')
                    }
                }
                stage('Test') {
                    steps {
                        githubCommitStatus(context: 'Test', description: 'Running tests...', status: 'PENDING')
                        sh 'cargo test --workspace'
                        githubCommitStatus(context: 'Test', description: 'Tests passed', status: 'SUCCESS')
                    }
                }
                stage('Documentation') {
                    steps {
                        githubCommitStatus(context: 'Docs', description: 'Building docs...', status: 'PENDING')
                        sh 'cargo doc --workspace --no-deps'
                        githubCommitStatus(context: 'Docs', description: 'Documentation built', status: 'SUCCESS')
                    }
                }
                stage('Build Release') {
                    when {
                        branch 'main'
                    }
                    steps {
                        githubCommitStatus(context: 'Release', description: 'Building release...', status: 'PENDING')
                        sh 'cargo build --workspace --release'
                        githubCommitStatus(context: 'Release', description: 'Release built', status: 'SUCCESS')
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
            githubCommitStatus(context: 'CI', description: 'All stages completed successfully', status: 'SUCCESS')
        }
        failure {
            echo 'Build failed!'
            githubCommitStatus(context: 'CI', description: 'Build failed', status: 'FAILURE')
        }
    }
}