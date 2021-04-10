import * as React from 'react'
import {useCallback, useEffect, useRef, useState} from "react";
import {Fragment, SystemEventData, SystemFacade} from "../SystemFacade";

function getLocalPos(e: any): {x: number, y: number} {
    const rect = e.target.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    return {x, y};
}

function SystemConsole() {
    const [system] = useState(() => new SystemFacade('ws://localhost:8080/ws/'))
    const prevPosRef = useRef<{x: number, y: number} | null>(null)
    const canvasRef = useRef<HTMLCanvasElement>(null)

    useEffect(() => {
        window.system = system;

        system.addEventListener('system', (e: any) => {
            const data = e.data as SystemEventData;
            if (data.SessionEvent?.Fragment) {
                draw(data.SessionEvent.Fragment)
            }
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
        system.sendFragment({
            x1: prevPosRef.current.x,
            y1: prevPosRef.current.y,
            x2: x,
            y2: y
        })
    }, [system])

    const draw = useCallback((fragment: Fragment) => {
        const {x1, y1, x2, y2} = fragment;
        const ctx = canvasRef.current.getContext('2d');
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
        prevPosRef.current = {x: x2, y: y2};
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