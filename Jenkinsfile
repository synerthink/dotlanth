pipeline {
    agent any
    
    // Define platforms to build on
    environment {
        PLATFORMS = ['linux', 'windows', 'macos']
        GITHUB_TOKEN = credentials('github-token')
    }
    
    stages {
        stage('Build and Test Matrix') {
            matrix {
                axes {
                    axis {
                        name 'PLATFORM'
                        values 'linux', 'windows', 'macos'
                    }
                }
                
                stages {
                    stage('Build and Test') {
                        agent {
                            docker {
                                image 'rust:latest'
                                args '-v cargo-cache:/usr/local/cargo/registry --user root'
                            }
                        }
                        
                        steps {
                            script {
                                // Set build status to pending
                                setBuildStatus("${PLATFORM} build/test in progress", "PENDING")
                                
                                try {
                                    // Setup stage
                                    stage('Setup Rust') {
                                        sh '''#!/bin/bash
                                            mkdir -p /usr/local/cargo/registry
                                            chmod -R 777 /usr/local/cargo/registry
                                            rustup default nightly
                                            rustup component add rustfmt clippy rust-src
                                        '''
                                    }

                                    // Format check stage
                                    stage('Check Format') {
                                        sh 'cargo fmt --all -- --check'
                                    }

                                    // Lint stage
                                    stage('Lint') {
                                        sh 'cargo clippy --workspace -- -D warnings'
                                    }

                                    // Build stage
                                    stage('Build') {
                                        sh 'cargo build --workspace'
                                    }

                                    // Test stage
                                    stage('Test') {
                                        sh 'cargo test --workspace'
                                    }

                                    // Documentation stage
                                    stage('Documentation') {
                                        sh 'cargo doc --workspace --no-deps'
                                    }

                                    // Release build for main branch
                                    if (env.BRANCH_NAME == 'main') {
                                        stage('Build Release') {
                                            sh 'cargo build --workspace --release'
                                        }
                                    }

                                    // Set success status
                                    setBuildStatus("${PLATFORM} build/test succeeded", "SUCCESS")
                                } catch (Exception e) {
                                    setBuildStatus("${PLATFORM} build/test failed", "FAILURE")
                                    throw e
                                }
                            }
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
            script {
                setBuildStatus("All builds succeeded", "SUCCESS")
                echo 'Build succeeded!'
            }
        }
        failure {
            script {
                setBuildStatus("Build failed", "FAILURE")
                echo 'Build failed!'
            }
        }
    }
}

// Function to set GitHub commit status
void setBuildStatus(String message, String state) {
    // Using curl to update GitHub status
    sh """
        curl -H "Authorization: token ${GITHUB_TOKEN}" \
             -X POST \
             -H "Accept: application/vnd.github.v3+json" \
             https://api.github.com/repos/${env.GITHUB_REPO}/statuses/${env.GIT_COMMIT} \
             -d '{
                 "state": "${state.toLowerCase()}", 
                 "target_url": "${env.BUILD_URL}", 
                 "description": "${message}", 
                 "context": "continuous-integration/jenkins"
             }'
    """
}

// Platform-specific configurations
def getPlatformConfig(platform) {
    def configs = [
        'linux': [
            image: 'rust:latest',
            shell: 'bash'
        ],
        'windows': [
            image: 'rust:latest-windowsservercore',
            shell: 'powershell'
        ],
        'macos': [
            image: 'rust:latest',
            shell: 'bash'
        ]
    ]
    return configs[platform]
}