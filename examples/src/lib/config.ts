// This module exports ENV_CONFIG, a string representing a configuration for
// talking to a y-sweet server. This is a JSON-stringified version of y-sweet's
// DocumentManagerOptions type. For ease of setting this value via an environment
// variable, the stringified version is accepted everywhere the
// DocumentManagerOptions type is.

export const ENV_CONFIG: string | undefined = process.env.Y_SWEET_CONFIG

if (ENV_CONFIG) {
  console.log('Using config from environment variable Y_SWEET_CONFIG')
} else {
  console.log('Using default config')
}
