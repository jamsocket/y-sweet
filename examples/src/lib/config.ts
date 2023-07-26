import { DocumentManagerOptions } from "./yserv";

export const ENV_CONFIG: DocumentManagerOptions = {
    endpoint: process.env.Y_SERVE_ENDPOINT,
    token: process.env.Y_SERVE_TOKEN,
}
