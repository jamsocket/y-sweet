export function Link(props: { href: string; children: React.ReactNode }) {
  return (
    <a href={props.href} className="text-pink-700 hover:text-pink-600 transition-colors underline">
      {props.children}
    </a>
  )
}
