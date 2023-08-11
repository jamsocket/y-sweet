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
        <ProjectLink
          name="Color Grid"
          url="/color-grid"
          description="Drop different shades of pink into a grid."
        />
        <ProjectLink
          name="To Do List"
          url="/to-do-list"
          description="Create and edit items in a to do list."
        />
        <ProjectLink
          name="Text Editor"
          url="/text-editor"
          description="A collaborative text editor built on top of the open-source Quill package."
        />
        <ProjectLink
          name="Code Editor"
          url="/code-editor"
          description="A collaborative code editor built on top of the open source CodeMirror package."
        />
        <ProjectLink
          name="Tree CRDT"
          url="/tree-crdt"
          description="A collaborative tree with reparenting."
        />
        <ProjectLink
          name="Voxel Draw"
          url="/voxels"
          description="A collaborative voxel drawing app."
        />
      </div>
    </div>
  )
}

interface ProjectLinkProps {
  name: string
  url: string
  description: string
}
function ProjectLink(props: ProjectLinkProps) {
  return (
    <Link
      href={props.url}
      className="flex flex-col pt-6 pb-8 px-6 bg-white/50 border border-white/80 shadow-sm hover:bg-white/90 transition-all rounded-lg"
    >
      <span className="text-pink-950 font-medium text-base lg:text-lg pb-2">{props.name}</span>
      <span className="text-pink-950 text-sm">{props.description}</span>
    </Link>
  )
}
