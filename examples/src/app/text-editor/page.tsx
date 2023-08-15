import { WrappedDocProvider } from '@/components/WrappedDocProvider'
import { TextEditor } from './TextEditor'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return <WrappedDocProvider searchParams={searchParams}>
    <TextEditor />
  </WrappedDocProvider>
}
