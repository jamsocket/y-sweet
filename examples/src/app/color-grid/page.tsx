import { ColorGrid } from './ColorGrid'
import { WrappedDocProvider } from '@/components/WrappedDocProvider'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return <WrappedDocProvider searchParams={searchParams}>
    <ColorGrid />
  </WrappedDocProvider>
}
