import { WrappedDocProvider } from '@/components/WrappedDocProvider'
import { Presence } from './Presence'

type HomeProps = {
  searchParams: Record<string, string>
}

export default async function Home({ searchParams }: HomeProps) {
  return (
    <WrappedDocProvider searchParams={searchParams}>
      <Presence />
    </WrappedDocProvider>
  )
}
