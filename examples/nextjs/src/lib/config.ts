// This module exports ENV_CONFIG, a string representing a configuration for
// talking to a y-sweet server. This is a JSON-stringified version of y-sweet's
// DocumentManagerOptions type. For ease of setting this value via an environment
// variable, the stringified version is accepted everywhere the
// DocumentManagerOptions type is.

export const CONNECTION_STRING: string = process.env.CONNECTION_STRING ?? 'ys://127.0.0.1:8080'

if (process.env.CONNECTION_STRING) {
  console.log('Using config from environment variable CONNECTION_STRING')
} else {
  console.log('Using default connection string, ys://127.0.0.1:8080')
}
