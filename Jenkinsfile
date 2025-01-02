pipeline {
    agent none

    options {
        buildDiscarder(logRotator(numToKeepStr: '10'))
        durabilityHint('PERFORMANCE_OPTIMIZED')
        githubProjectProperty(displayName: 'dotVM', projectUrlStr: 'https://github.com/synerthink-organization/dotVM/')
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
                                githubNotify context: "ci/jenkins/${PLATFORM}", 
                                            description: "Build started on ${PLATFORM}",
                                            status: 'PENDING'
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
                                githubNotify context: "ci/jenkins/${PLATFORM}",
                                            description: "Build succeeded on ${PLATFORM}",
                                            status: 'SUCCESS'
                            }
                            failure {
                                githubNotify context: "ci/jenkins/${PLATFORM}",
                                            description: "Build failed on ${PLATFORM}",
                                            status: 'FAILURE'
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