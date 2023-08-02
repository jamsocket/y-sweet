import Link from 'next/link'
import Title from '@/components/Title'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return (
    <div className="space-y-4 p-4 lg:p-8">
      <Title>Demos</Title>
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 max-w-2xl">
        <ProjectLink name="Color Grid" url="/color-grid" />
        <ProjectLink name="To Do List" url="/to-do-list" />
        <ProjectLink name="Text Editor" url="/text-editor" />
        <ProjectLink name="Code Editor" url="/code-editor" />
      </div>
    </div>
  )
}

interface ProjectLinkProps {
  name: string
  url: string
}
function ProjectLink(props: ProjectLinkProps) {
  return (
    <Link
      href={props.url}
      className="text-pink-950 text-base lg:text-lg font-medium bg-white/50 border border-white/80 shadow-sm hover:bg-white/90 transition-all text-center py-6 lg:py-10 rounded-lg"
    >
      {props.name}
    </Link>
  )
}
