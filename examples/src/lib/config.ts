import { DocumentManagerOptions } from './yserv'

export const ENV_CONFIG: DocumentManagerOptions = {
  endpoint: process.env.Y_SWEET_ENDPOINT,
  token: process.env.Y_SWEET_API_TOKEN,
}
