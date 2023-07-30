export const ENV_CONFIG: string | undefined = process.env.Y_SWEET_CONFIG

if (ENV_CONFIG) {
  console.log('Using config from environment variable Y_SWEET_CONFIG')
} else {
  console.log('Using default config')
}
