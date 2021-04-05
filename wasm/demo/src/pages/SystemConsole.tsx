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
    const [drawing, setDrawing] = useState<{x: number, y: number} | null>(null)
    const canvasRef = useRef<HTMLCanvasElement>(null)

    const handleMouseDown = useCallback(e => {
        console.log("mouse down")
        const {x, y} = getLocalPos(e);
        setDrawing({x, y});
    }, [])

    const handleMouseUp = useCallback(e => {
        console.log("mouse up")
        setDrawing(null);
    }, [])

    const handleMouseMove = useCallback(e => {
        if (system && drawing) {
            const {x, y} = getLocalPos(e);
            const buf = system.translate_from_json(JSON.stringify({Fragment: {x1: drawing.x, y1: drawing.y, x2: x, y2: y}}));
            const json = JSON.parse(system.translate_to_json(buf));
            if (json.Fragment) {
                const ctx = canvasRef.current.getContext('2d');
                ctx.moveTo(json.Fragment.x1, json.Fragment.y1);
                ctx.lineTo(json.Fragment.x2, json.Fragment.y2);
                ctx.stroke();
            }
            setDrawing({x, y});
        }
    }, [system, drawing])

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