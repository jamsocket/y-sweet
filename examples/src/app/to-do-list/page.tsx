import { ToDoList } from './ToDoList'
import { WrappedDocProvider } from '@/components/WrappedDocProvider'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return (
    <WrappedDocProvider searchParams={searchParams}>
      <ToDoList />
    </WrappedDocProvider>
  )
}
