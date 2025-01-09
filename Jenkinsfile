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
        // Required checks that must pass before merging (removed coverage)
        REQUIRED_CONTEXTS = [
            'format',
            'lint',
            'test',
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
                        values 'linux' // Temporarily only Linux until cross-platform is needed
                    }
                }
                
                stages {
                    stage('Verify Docker Image') {
                        steps {
                            sh 'docker pull rust:slim'
                        }
                    }
                    
                    stage('Build and Test') {
                        agent {
                            docker {
                                image 'rust:slim'
                                args '-v cargo-cache:/usr/local/cargo/registry --user root'
                                reuseNode true
                            }
                        }
                        
                        steps {
                            // Install required packages
                            sh '''
                                apt-get update && apt-get install -y curl pkg-config libssl-dev
                                rustup default nightly
                                rustup component add rustfmt clippy rust-src
                            '''
                            
                            script {
                                // Format check
                                try {
                                    sh 'cargo fmt --all -- --check'
                                    updateGithubStatus('format', 'success', 'Format check passed')
                                } catch (Exception e) {
                                    updateGithubStatus('format', 'failure', 'Format check failed')
                                    error('Format check failed')
                                }

                                // Lint check
                                try {
                                    sh 'cargo clippy --workspace -- -D warnings'
                                    updateGithubStatus('lint', 'success', 'Lint check passed')
                                } catch (Exception e) {
                                    updateGithubStatus('lint', 'failure', 'Lint check failed')
                                    error('Lint check failed')
                                }

                                // Build and test
                                try {
                                    sh 'cargo build --workspace'
                                    sh 'cargo test --workspace'
                                    updateGithubStatus('test', 'success', 'Tests passed')
                                } catch (Exception e) {
                                    updateGithubStatus('test', 'failure', 'Tests failed')
                                    error('Tests failed')
                                }

                                // Documentation
                                try {
                                    sh 'cargo doc --workspace --no-deps'
                                    archiveArtifacts artifacts: 'target/doc/**/*', fingerprint: true
                                    updateGithubStatus('documentation', 'success', 'Documentation built successfully')
                                } catch (Exception e) {
                                    updateGithubStatus('documentation', 'failure', 'Documentation build failed')
                                    error('Documentation build failed')
                                }

                                // Release build if on main branch
                                if (env.BRANCH_NAME == 'main') {
                                    sh 'cargo build --workspace --release'
                                }
                            }
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