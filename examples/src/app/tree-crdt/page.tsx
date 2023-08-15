import { WrappedDocProvider } from '@/components/WrappedDocProvider'
import { TreeView } from './TreeView'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return <WrappedDocProvider searchParams={searchParams}>
    <TreeView />
  </WrappedDocProvider>
}
