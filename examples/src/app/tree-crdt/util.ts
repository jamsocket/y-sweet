export function randomString() {
    return Math.random().toString(36).substring(2, 7)
}

export function randomColor() {
    const hue = Math.random() * 360
    const value = Math.random() * 0.5 + 0.5
    return `hsl(${hue}, 100%, ${value * 100}%)`
}
