"use client"

import { useText } from "@/lib/provider"
import {useState} from "react"
import ReactQuill from 'react-quill';
import 'react-quill/dist/quill.snow.css';

import 'quill/dist/quill.bubble.css';

export function TextEditor() {
    const yText = useText('text')


    return <ReactQuill theme="snow" value={yText?.toString()} onChange={(delta) => yText?.applyDelta(delta)} />;
}
