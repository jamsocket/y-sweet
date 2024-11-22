const COLORS = [
  "#FF5C5C",
  "#FFB65C",
  "#88FF70",
  "#47F0FF",
  "#478EFF",
  "#745CFF",
  "#FF85FF",
];

export function randomColor() {
  return COLORS[Math.floor(Math.random() * COLORS.length)];
}
