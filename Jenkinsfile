pipeline {
    agent any
    
    options {
        buildDiscarder(logRotator(numToKeepStr: '10'))
        durabilityHint('PERFORMANCE_OPTIMIZED')
        githubProjectProperty(
            displayName: 'dotVM',
            projectUrlStr: 'https://github.com/synerthink-organization/dotVM/'
        )
    }
    
    stages {
        stage('Matrix Build') {
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
                                withCredentials([string(credentialsId: 'github-token', variable: 'GITHUB_TOKEN')]) {
                                    try {
                                        sh """
                                            curl -H "Authorization: token ${GITHUB_TOKEN}" \
                                            -X POST \
                                            -H "Accept: application/vnd.github.v3+json" \
                                            https://api.github.com/repos/synerthink-organization/dotVM/statuses/\${GIT_COMMIT} \
                                            -d '{
                                                "state": "pending",
                                                "target_url": "${BUILD_URL}",
                                                "description": "Build started on ${PLATFORM}",
                                                "context": "ci/jenkins/${PLATFORM}"
                                            }'
                                        """
                                    } catch (Exception e) {
                                        echo "Failed to update GitHub status: ${e.message}"
                                    }
                                }
                            }
                            
                            sh '''#!/bin/bash
                                mkdir -p /usr/local/cargo/registry
                                chmod -R 777 /usr/local/cargo/registry
                                rustup default nightly
                                rustup component add rustfmt clippy rust-src
                            '''
                            
                            sh 'cargo fmt --all -- --check'
                            sh 'cargo clippy --workspace -- -D warnings'
                            sh 'cargo build --workspace'
                            sh 'cargo test --workspace'
                            sh 'cargo doc --workspace --no-deps'
                            
                            script {
                                if (env.BRANCH_NAME == 'main') {
                                    sh 'cargo build --workspace --release'
                                }
                            }
                        }
                        
                        post {
                            success {
                                script {
                                    withCredentials([string(credentialsId: 'github-token', variable: 'GITHUB_TOKEN')]) {
                                        try {
                                            sh """
                                                curl -H "Authorization: token ${GITHUB_TOKEN}" \
                                                -X POST \
                                                -H "Accept: application/vnd.github.v3+json" \
                                                https://api.github.com/repos/synerthink-organization/dotVM/statuses/\${GIT_COMMIT} \
                                                -d '{
                                                    "state": "success",
                                                    "target_url": "${BUILD_URL}",
                                                    "description": "Build succeeded on ${PLATFORM}",
                                                    "context": "ci/jenkins/${PLATFORM}"
                                                }'
                                            """
                                        } catch (Exception e) {
                                            echo "Failed to update GitHub status: ${e.message}"
                                        }
                                    }
                                }
                            }
                            failure {
                                script {
                                    withCredentials([string(credentialsId: 'github-token', variable: 'GITHUB_TOKEN')]) {
                                        try {
                                            sh """
                                                curl -H "Authorization: token ${GITHUB_TOKEN}" \
                                                -X POST \
                                                -H "Accept: application/vnd.github.v3+json" \
                                                https://api.github.com/repos/synerthink-organization/dotVM/statuses/\${GIT_COMMIT} \
                                                -d '{
                                                    "state": "failure",
                                                    "target_url": "${BUILD_URL}",
                                                    "description": "Build failed on ${PLATFORM}",
                                                    "context": "ci/jenkins/${PLATFORM}"
                                                }'
                                            """
                                        } catch (Exception e) {
                                            echo "Failed to update GitHub status: ${e.message}"
                                        }
                                    }
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
    }
}