pipeline {
    agent none
    
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
                            withCredentials([string(credentialsId: 'github-token', variable: 'GITHUB_TOKEN')]) {
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
                                // Set success status using GitHub API
                                withCredentials([string(credentialsId: 'github-token', variable: 'GITHUB_TOKEN')]) {
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
                                }
                            }
                            failure {
                                withCredentials([string(credentialsId: 'github-token', variable: 'GITHUB_TOKEN')]) {
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