pipeline {
    agent any
    
    // Define build platforms
    environment {
        REPO_OWNER = 'synerthink-organization'
        REPO_NAME = 'dotVM'
        BUILD_PLATFORMS = 'linux,windows,macos'  // Comma-separated string for environment variable
    }
    
    stages {
        stage('Matrix Build') {
            matrix {
                agent {
                    docker {
                        image 'rust:latest'
                        args '-v cargo-cache:/usr/local/cargo/registry --user root'
                    }
                }
                
                axes {
                    axis {
                        name 'PLATFORM'
                        values 'linux', 'windows', 'macos'
                    }
                }
                
                stages {
                    stage('Set Pending Status') {
                        steps {
                            sshagent(['jenkinssh']) {
                                sh """
                                    curl -i -H "Accept: application/vnd.github.v3+json" \
                                        --user "${REPO_OWNER}:${SSH_KEY}" \
                                        -X POST \
                                        -d '{"state": "pending", "target_url": "${env.BUILD_URL}", "description": "Build is pending", "context": "ci/jenkins/${PLATFORM}"}' \
                                        https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/statuses/${GIT_COMMIT}
                                """
                            }
                        }
                    }
                    
                    stage('Setup Rust') {
                        steps {
                            sh '''#!/bin/bash
                                mkdir -p /usr/local/cargo/registry
                                chmod -R 777 /usr/local/cargo/registry
                                rustup default nightly
                                rustup component add rustfmt clippy rust-src
                                rustup target add x86_64-unknown-linux-gnu
                                rustup target add x86_64-pc-windows-gnu
                                rustup target add x86_64-apple-darwin
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
                    
                    stage('Platform Build') {
                        steps {
                            script {
                                def target
                                switch(PLATFORM) {
                                    case 'linux':
                                        target = 'x86_64-unknown-linux-gnu'
                                        break
                                    case 'windows':
                                        target = 'x86_64-pc-windows-gnu'
                                        break
                                    case 'macos':
                                        target = 'x86_64-apple-darwin'
                                        break
                                }
                                sh "cargo build --workspace --target ${target}"
                            }
                        }
                    }
                    
                    stage('Test') {
                        steps {
                            script {
                                def target
                                switch(PLATFORM) {
                                    case 'linux':
                                        target = 'x86_64-unknown-linux-gnu'
                                        break
                                    case 'windows':
                                        target = 'x86_64-pc-windows-gnu'
                                        break
                                    case 'macos':
                                        target = 'x86_64-apple-darwin'
                                        break
                                }
                                sh "cargo test --workspace --target ${target}"
                            }
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
                            script {
                                def target
                                switch(PLATFORM) {
                                    case 'linux':
                                        target = 'x86_64-unknown-linux-gnu'
                                        break
                                    case 'windows':
                                        target = 'x86_64-pc-windows-gnu'
                                        break
                                    case 'macos':
                                        target = 'x86_64-apple-darwin'
                                        break
                                }
                                sh "cargo build --workspace --release --target ${target}"
                            }
                        }
                    }
                }
                
                post {
                    success {
                        sshagent(['jenkinssh']) {
                            sh """
                                curl -i -H "Accept: application/vnd.github.v3+json" \
                                    --user "${REPO_OWNER}:${SSH_KEY}" \
                                    -X POST \
                                    -d '{"state": "success", "target_url": "${env.BUILD_URL}", "description": "Build succeeded", "context": "ci/jenkins/${PLATFORM}"}' \
                                    https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/statuses/${GIT_COMMIT}
                            """
                        }
                    }
                    failure {
                        sshagent(['jenkinssh']) {
                            sh """
                                curl -i -H "Accept: application/vnd.github.v3+json" \
                                    --user "${REPO_OWNER}:${SSH_KEY}" \
                                    -X POST \
                                    -d '{"state": "failure", "target_url": "${env.BUILD_URL}", "description": "Build failed", "context": "ci/jenkins/${PLATFORM}"}' \
                                    https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}/statuses/${GIT_COMMIT}
                            """
                        }
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
            echo 'All builds succeeded!'
        }
        failure {
            echo 'Some builds failed!'
        }
    }
}