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

    environment {
        // Required checks that must pass before merging
        REQUIRED_CONTEXTS = [
            'format',
            'lint',
            'test',
            'coverage',
            'documentation'
        ].join(' ')
    }

    stages {
        stage('Initialization') {
            steps {
                script {
                    // Set initial status for all required checks
                    if (env.CHANGE_ID) { // If this is a PR
                        REQUIRED_CONTEXTS.split(' ').each { context ->
                            updateGithubStatus(context, 'pending', "Check pending for ${context}")
                        }
                    }
                }
            }
        }

        stage('Matrix Build') {
            matrix {
                axes {
                    axis {
                        name 'PLATFORM'
                        values 'linux', 'windows', 'macos'
                    }
                }

                stages {
                    stage('Setup') {
                        steps {
                            script {
                                sh '''#!/bin/bash
                                    mkdir -p $HOME/.cargo/registry
                                    chmod -R 777 $HOME/.cargo/registry
                                    
                                    # Install Rust if not present
                                    if ! command -v rustup &> /dev/null; then
                                        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                                        source $HOME/.cargo/env
                                    fi
                                    
                                    # Set up toolchain
                                    $HOME/.cargo/bin/rustup default nightly
                                    $HOME/.cargo/bin/rustup component add rustfmt clippy rust-src
                                    $HOME/.cargo/bin/cargo install cargo-tarpaulin
                                '''
                            }
                        }
                    }

                    stage('Format Check') {
                        when { equals expected: 'linux', actual: env.PLATFORM }
                        steps {
                            script {
                                try {
                                    sh 'cargo fmt --all -- --check'
                                    updateGithubStatus('format', 'success', 'Format check passed')
                                } catch (Exception e) {
                                    updateGithubStatus('format', 'failure', 'Format check failed')
                                    error('Format check failed')
                                }
                            }
                        }
                    }

                    stage('Lint') {
                        when { equals expected: 'linux', actual: env.PLATFORM }
                        steps {
                            script {
                                try {
                                    sh 'cargo clippy --workspace -- -D warnings'
                                    updateGithubStatus('lint', 'success', 'Lint check passed')
                                } catch (Exception e) {
                                    updateGithubStatus('lint', 'failure', 'Lint check failed')
                                    error('Lint check failed')
                                }
                            }
                        }
                    }

                    stage('Build and Test') {
                        steps {
                            script {
                                try {
                                    sh 'cargo build --workspace'
                                    sh 'cargo test --workspace'
                                    updateGithubStatus('test', "success", "Tests passed on ${PLATFORM}")
                                } catch (Exception e) {
                                    updateGithubStatus('test', 'failure', "Tests failed on ${PLATFORM}")
                                    error("Tests failed on ${PLATFORM}")
                                }
                            }
                        }
                    }

                    stage('Coverage') {
                        when { equals expected: 'linux', actual: env.PLATFORM }
                        steps {
                            script {
                                try {
                                    sh '''
                                        cargo tarpaulin --workspace --coverage-threshold 80 \
                                            --out Xml --out Html --output-dir coverage
                                    '''
                                    archiveArtifacts artifacts: 'coverage/**/*', fingerprint: true
                                    updateGithubStatus('coverage', 'success', 'Coverage check passed')
                                } catch (Exception e) {
                                    updateGithubStatus('coverage', 'failure', 'Coverage check failed')
                                    error('Coverage check failed')
                                }
                            }
                        }
                    }

                    stage('Documentation') {
                        when { equals expected: 'linux', actual: env.PLATFORM }
                        steps {
                            script {
                                try {
                                    sh 'cargo doc --workspace --no-deps'
                                    archiveArtifacts artifacts: 'target/doc/**/*', fingerprint: true
                                    updateGithubStatus('documentation', 'success', 'Documentation built successfully')
                                } catch (Exception e) {
                                    updateGithubStatus('documentation', 'failure', 'Documentation build failed')
                                    error('Documentation build failed')
                                }
                            }
                        }
                    }

                    stage('Release Build') {
                        when {
                            allOf {
                                equals expected: 'linux', actual: env.PLATFORM
                                branch 'main'
                            }
                        }
                        steps {
                            sh 'cargo build --workspace --release'
                        }
                    }
                }
            }
        }

        stage('PR Status Check') {
            when { expression { env.CHANGE_ID != null } }
            steps {
                script {
                    def allChecksPass = REQUIRED_CONTEXTS.split(' ').every { context ->
                        def statusUrl = "https://api.github.com/repos/synerthink-organization/dotVM/commits/${GIT_COMMIT}/statuses"
                        def response = sh(
                            script: """
                                curl -H "Authorization: token ${GITHUB_TOKEN}" \
                                -H "Accept: application/vnd.github.v3+json" \
                                ${statusUrl}
                            """,
                            returnStdout: true
                        )
                        def statuses = readJSON(text: response)
                        return statuses.find { it.context == "ci/jenkins/${context}" }?.state == 'success'
                    }

                    if (!allChecksPass) {
                        error('Not all required checks have passed')
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
                if (env.CHANGE_ID) {
                    setGithubPRStatus('success', 'All checks have passed')
                }
            }
        }
        failure {
            script {
                if (env.CHANGE_ID) {
                    setGithubPRStatus('failure', 'Some checks have failed')
                }
            }
        }
    }
}

// Helper function to update GitHub commit status
void updateGithubStatus(String context, String state, String description) {
    withCredentials([string(credentialsId: 'GIT_TOKEN', variable: 'GITHUB_TOKEN')]) {
        sh """
            curl -H "Authorization: token ${GITHUB_TOKEN}" \
            -X POST \
            -H "Accept: application/vnd.github.v3+json" \
            https://api.github.com/repos/synerthink-organization/dotVM/statuses/\${GIT_COMMIT} \
            -d '{
                "state": "${state}",
                "target_url": "${BUILD_URL}",
                "description": "${description}",
                "context": "ci/jenkins/${context}"
            }'
        """
    }
}

// Helper function to update GitHub PR status
void setGithubPRStatus(String state, String description) {
    withCredentials([string(credentialsId: 'GIT_TOKEN', variable: 'GITHUB_TOKEN')]) {
        sh """
            curl -H "Authorization: token ${GITHUB_TOKEN}" \
            -X POST \
            -H "Accept: application/vnd.github.v3+json" \
            https://api.github.com/repos/synerthink-organization/dotVM/statuses/\${GIT_COMMIT} \
            -d '{
                "state": "${state}",
                "target_url": "${BUILD_URL}",
                "description": "${description}",
                "context": "ci/jenkins/pr-check"
            }'
        """
    }
}