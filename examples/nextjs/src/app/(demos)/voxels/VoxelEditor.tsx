'use client'

import { OrbitControls } from '@react-three/drei'
import { Canvas, ThreeEvent, useFrame } from '@react-three/fiber'
import { Compact } from '@uiw/react-color'
import { usePresence, useMap, usePresenceSetter } from '@y-sweet/react'
import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import { Vector3, Vector3Tuple } from 'three'

const DIM = 15
const TRANSITION_RATE = 0.08

interface Voxel {
  position: Vector3Tuple
  color: any
  opacity: number
}

function MovingVoxel(props: { voxel: Voxel; name?: string }) {
  const [position, setPosition] = useState<Vector3Tuple>([0, 0, 0]) // this gets set before the first paint
  const destPositionRef = useRef<Vector3Tuple | null>(null)

  useFrame(() => {
    if (destPositionRef.current) {
      const delta = [
        destPositionRef.current[0] - position[0],
        destPositionRef.current[1] - position[1],
        destPositionRef.current[2] - position[2],
      ]

      const x = position[0] + delta[0] * TRANSITION_RATE
      const y = position[1] + delta[1] * TRANSITION_RATE
      const z = position[2] + delta[2] * TRANSITION_RATE

      setPosition([x, y, z])

      const squaredDist = delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]
      if (squaredDist < 0.00001) {
        destPositionRef.current = null
      }
    }
  })

  useLayoutEffect(() => {
    const dest: Vector3Tuple = [
      props.voxel.position[0],
      props.voxel.position[1] + 0.5,
      props.voxel.position[2],
    ]

    const squaredDist =
      Math.pow(dest[0] - position[0], 2) +
      Math.pow(dest[1] - position[1], 2) +
      Math.pow(dest[2] - position[2], 2)
    if (squaredDist > 0.00001) {
      destPositionRef.current = dest
    }
  }, [props.voxel.position])

  return (
    <mesh name={props.name} position={position} scale={1}>
      <boxGeometry args={[1, 1, 1]} />

      <meshPhongMaterial
        color={props.voxel.color}
        opacity={props.voxel.opacity}
        transparent={props.voxel.opacity < 1}
      />
    </mesh>
  )
}

function Voxel(props: { voxel: Voxel; name?: string }) {
  const position: Vector3Tuple = [
    props.voxel.position[0],
    props.voxel.position[1] + 0.5,
    props.voxel.position[2],
  ]
  return (
    <mesh
      name={props.name}
      position={position}
      scale={1}
      castShadow={props.voxel.opacity === 1}
      receiveShadow={true}
    >
      <boxGeometry args={[1, 1, 1]} />

      <meshPhongMaterial
        color={props.voxel.color}
        opacity={props.voxel.opacity}
        transparent={props.voxel.opacity < 1}
      />
    </mesh>
  )
}

function getPosition(event: ThreeEvent<PointerEvent>): Vector3Tuple | null {
  if (event.intersections.length === 0) return null

  const { face, point } = event.intersections[0]
  const normal: Vector3 = face!.normal.clone()

  const pos: Vector3 = point.clone().add(new Vector3(0.5, 0.0, 0.5))

  const c = pos.add(normal.multiplyScalar(0.5)).floor()
  return c.toArray()
}

interface VoxelSetProps {
  voxels: Record<string, Voxel>
}

function VoxelSet(props: VoxelSetProps) {
  return (
    <>
      {Object.entries(props.voxels).map(([index, voxel]) => (
        <Voxel key={index} voxel={voxel} name={index} />
      ))}
    </>
  )
}

type PresenceVoxel = {
  position: [number, number, number] | null
  color: string
}

export function PresenceVoxels() {
  const presence = usePresence<PresenceVoxel>()

  return (
    <>
      {Array.from(presence.entries()).map(([id, user]) => {
        if (user.position === null) return null

        return (
          <MovingVoxel
            key={id}
            voxel={{ position: user.position, color: user.color, opacity: 0.5 }}
          />
        )
      })}
    </>
  )
}

export function VoxelEditor() {
  const [ghostPosition, setGhostPosition] = useState<[number, number, number] | null>(null)
  const voxels = useMap<Voxel>('voxels')
  const [color, setColor] = useState('#D33115')

  const positionHasBeenSet = useRef(false)
  const setInitialCameraPosition = (controls: any) => {
    if (controls && !positionHasBeenSet.current) {
      controls.object.position.set(0, DIM, DIM)
      positionHasBeenSet.current = true
    }
  }

  const updatePresence = usePresenceSetter<PresenceVoxel>()

  const pointerMove = useCallback(
    (event: ThreeEvent<PointerEvent>) => {
      const position = getPosition(event)
      setGhostPosition(position)

      updatePresence({
        position: position,
        color: color,
      })
    },
    [setGhostPosition],
  )

  const handleClick = useCallback(
    (event: ThreeEvent<MouseEvent>) => {
      if (event.delta > 5) {
        // ignore drag events, which are handled by the orbit control
        return
      }

      if (event.shiftKey) {
        if (event.object.name) {
          voxels.delete(event.object.name)
        }

        event.stopPropagation()
        return
      }

      const position = getPosition(event as any)
      if (position) {
        voxels.set(position.join(':'), { color, position, opacity: 1 })
      }

      event.stopPropagation()
    },
    [color, voxels],
  )

  let voxelArray: Record<string, Voxel> = voxels.toJSON()

  return (
    <>
      <div style={{ position: 'absolute', top: 0, right: 0, left: 0, bottom: 0 }}>
        <Canvas shadows>
          <OrbitControls ref={setInitialCameraPosition} />
          <ambientLight intensity={2} />
          <pointLight position={[0, 10, 0]} intensity={100} castShadow />
          <pointLight position={[10, 10, 10]} intensity={100} castShadow />
          <pointLight position={[10, 10, 0]} intensity={100} castShadow />

          {ghostPosition ? (
            <Voxel voxel={{ position: ghostPosition, color: 0x000000, opacity: 0.5 }} />
          ) : null}

          <PresenceVoxels />

          <gridHelper args={[DIM, DIM]} position={[0, 0.001, 0]} />

          <group onPointerMove={pointerMove} onClick={handleClick}>
            <mesh scale={1} position={[0, -0.05, 0]} castShadow receiveShadow>
              <boxGeometry args={[DIM, 0.1, DIM]} />
              <meshPhongMaterial color="#eee" opacity={1} />
            </mesh>
            <VoxelSet voxels={voxelArray} />
          </group>
        </Canvas>
      </div>
      <Compact color={color} onChange={(color) => setColor(color.hex)} />
    </>
  )
}
