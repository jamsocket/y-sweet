import Link from 'next/link'
import Title from '@/components/Title'

export default function Home() {
  return (
    <div className="lg:h-full overflow-y-scroll space-y-4 p-4 lg:p-8 relative w-auto bg-[radial-gradient(at_bottom_left,_var(--tw-gradient-stops))] from-white/90 via-pink-50/90 to-pink-100/90 rounded-lg">
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
          name="Text Editor with Quill"
          url="/text-editor"
          description="A collaborative text editor built on top of the open-source Quill package."
        />
        <ProjectLink
          name="Text Editor with Slate"
          url="/slate"
          description="A collaborative text editor built on top of the open-source Slate package."
        />
        <ProjectLink
          name="Code Editor with CodeMirror"
          url="/code-editor"
          description="A collaborative code editor built on top of the open source CodeMirror package."
        />
        <ProjectLink
          name="Code Editor with Monaco"
          url="/monaco"
          description="A collaborative code editor built with y-monaco."
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
        <ProjectLink
          name="BlockNote"
          url="/blocknote"
          description="A collaborative block-based rich text editor using the open-source BlockNote package."
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
