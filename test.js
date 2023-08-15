
async function main() {
    try {
        await fetch("https://example.com/")
    } catch (error) {
        console.error(error.cause)
    }    
}

main().then(() => console.log("done")).catch(() => console.log("error"))
