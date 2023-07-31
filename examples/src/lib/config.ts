import type { DocumentManagerOptions } from '@y-sweet/sdk'

// This module exports ENV_CONFIG, a string representing a configuration for
// talking to a y-sweet server. This is a JSON-stringified version of y-sweet's
// DocumentManagerOptions type.
export const ENV_CONFIG: DocumentManagerOptions | undefined = process.env.Y_SWEET_CONFIG
  ? JSON.parse(process.env.Y_SWEET_CONFIG)
  : undefined

if (ENV_CONFIG) {
  console.log('Using config from environment variable Y_SWEET_CONFIG')
  console.log(`Endpoint: ${ENV_CONFIG.endpoint}`)
  console.log(ENV_CONFIG.token ? 'Token provided' : 'No token provided')
} else {
  console.log('Using default config')
}
