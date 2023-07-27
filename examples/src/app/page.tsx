import Link from 'next/link'
import Title from '@/components/Title'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return (
    <div className="space-y-4 m-10">
      <Title>Demos</Title>
      <div className='grid grid-cols-2 gap-4'>
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
function ProjectLink(props:ProjectLinkProps) {
  return(
    <Link href={props.url} className="text-pink-950 hover:bg-white transition-all p-10 border-2 border-pink-950 rounded-lg">
    {props.name}
  </Link>
  )
}
