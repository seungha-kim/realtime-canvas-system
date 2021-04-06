import * as React from 'react'
import {useCallback, useEffect, useRef, useState} from "react";

// @ts-ignore
import mod from '../../../pkg/realtime_canvas_bg.wasm'
import init, {CanvasSystem} from '../../../pkg/realtime_canvas.js'

function useSystem() {
    const [system, setSystem] = useState<CanvasSystem | undefined>()

    useEffect(() => {
        init(mod).then(() => {
            setSystem(new CanvasSystem())
        })
    }, [])

    return system
}

function getLocalPos(e: any): {x: number, y: number} {
    const rect = e.target.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    return {x, y};
}

function SystemConsole() {
    const system = useSystem()
    const prevPosRef = useRef<{x: number, y: number} | null>(null)
    const wsRef = useRef<WebSocket>()
    const canvasRef = useRef<HTMLCanvasElement>(null)

    useEffect(() => {
        if (!system) return
        const ws = new WebSocket('ws://localhost:8080/ws/')
        ws.binaryType = 'arraybuffer'
        ws.onclose = ws.onerror = ws.onmessage = ws.onopen = console.log
        wsRef.current = ws
        ws.addEventListener('message', e => {
            const buf = new Uint8Array(e.data)
            const json = system.translate_to_json(buf);
            const parsed = JSON.parse(json);
            draw(parsed)
        })
    }, [system])

    const handleMouseDown = useCallback(e => {
        console.log("mouse down")
        const {x, y} = getLocalPos(e);
        prevPosRef.current = {x, y};
    }, [])

    const handleMouseUp = useCallback(e => {
        console.log("mouse up")
        prevPosRef.current = null;
    }, [])

    const sendFragment = useCallback((x: number, y: number) => {
        const json = JSON.stringify({
            Fragment: {
                x1: prevPosRef.current.x,
                y1: prevPosRef.current.y,
                x2: x,
                y2: y
            }
        })
        const buf = system.translate_from_json(json);
        wsRef.current?.send(buf)
    }, [system])

    const draw = useCallback((parsed: any) => {
        if (parsed.Fragment) {
            const {x1, y1, x2, y2} = parsed.Fragment;
            const ctx = canvasRef.current.getContext('2d');
            ctx.moveTo(x1, y1);
            ctx.lineTo(x2, y2);
            ctx.stroke();
            prevPosRef.current = {x: x2, y: y2};
        }
    }, [])

    const handleMouseMove = useCallback(e => {
        if (system && prevPosRef.current) {
            const {x, y} = getLocalPos(e);
            sendFragment(x, y);
        }
    }, [system])

    if (!system) {
        return <div>Loading</div>
    } else {
        return <div>
            <canvas ref={canvasRef} width={100} height={100} style={{width:100, height:100, border: '1px solid silver'}}
            onMouseDown={handleMouseDown} onMouseMove={handleMouseMove} onMouseUp={handleMouseUp} />
        </div>
    }
}

export default SystemConsole