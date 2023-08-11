'use client'

import { OrbitControls } from '@react-three/drei'
import { Canvas, ThreeEvent, useFrame } from '@react-three/fiber'
import { useCallback, useLayoutEffect, useRef, useState } from 'react'
import { Vector3, Vector3Tuple } from 'three'
import * as Y from 'yjs'
import { usePresence } from '@y-sweet/react'
import { CompactPicker } from 'react-color'

const DIM = 15
const TRANSITION_RATE = 0.8

export function VoxelEditor() {
  return (
    <>
      <div style={{ position: 'absolute', top: 0, right: 0, left: 0, bottom: 0 }}>
        <h1> YO BOIS </h1>
      </div>
    </>
  )
}
