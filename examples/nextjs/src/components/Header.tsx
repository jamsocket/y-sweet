'use client'
import Title from '@/components/Title'

interface HeaderProps {
  title: string
  githubLink: string
}
export default function Header(props: HeaderProps) {
  const { title, githubLink } = props
  return (
    <div className="flex flex-col md:flex-row justify-between md:items-center gap-y-2">
      <Title>{title}</Title>
      <div className="flex">
        <a
          className="text-sm flex gap-2 items-center px-3 py-2 rounded-lg bg-pink-950 text-white border transition-all "
          href={githubLink}
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="currentColor"
            className="w-5 h-5 text-white"
          >
            <path
              fillRule="evenodd"
              d="M12 0C5.373 0 0 5.373 0 12c0 5.302 3.438 9.8 8.205 11.387.6.11.793-.26.793-.577v-2.234c-3.338.726-4.033-1.415-4.033-1.415-.546-1.385-1.333-1.753-1.333-1.753-1.089-.744.083-.729.083-.729 1.205.084 1.838 1.238 1.838 1.238 1.07 1.834 2.809 1.305 3.495.998.108-.774.418-1.305.762-1.604-2.665-.3-5.466-1.333-5.466-5.931 0-1.31.469-2.382 1.236-3.221-.124-.303-.535-1.522.117-3.176 0 0 1.008-.322 3.3 1.23a11.48 11.48 0 013.004-.404c1.019.005 2.047.138 3.004.404 2.292-1.552 3.3-1.23 3.3-1.23.653 1.654.242 2.873.118 3.176.769.839 1.236 1.911 1.236 3.221 0 4.61-2.807 5.628-5.478 5.921.429.372.812 1.102.812 2.222v3.293c0 .319.192.694.801.576C20.565 21.796 24 17.298 24 12c0-6.627-5.373-12-12-12z"
              clipRule="evenodd"
            />
          </svg>
          <span>See code on Github</span>
        </a>
      </div>
    </div>
  )
}
