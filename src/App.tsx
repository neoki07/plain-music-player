import { useEffect, useRef, useState } from "react";
import {
  IoPlaySharp,
  IoPlayForwardSharp,
  IoPlayBackSharp,
  IoMusicalNotesSharp,
} from "react-icons/io5";
import { open } from "@tauri-apps/api/dialog";
import { invoke } from '@tauri-apps/api/tauri'

type Progress = [number, number, number];

function formatTime(time: number) {
  const min = Math.floor(time / 60);
  const sec = time % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

function App() {
  const divRef = useRef<HTMLDivElement>(null);
  const [jacketSize, setJacketSize] = useState(0);
  const [progress, setProgress] = useState<Progress>([0, 0, 0]);

  const requestIdRef = useRef(0);
  const animate = () => {
    invoke("get_progress").then((progress) => setProgress(progress as Progress));
    requestIdRef.current = requestAnimationFrame(animate)
  };
  useEffect(() => {
    requestIdRef.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(requestIdRef.current);
  }, [])

  function openDialog() {
    open().then((files) => {
      if (files && typeof files == "string") {
        console.log("files:", files);
        invoke("play", {path: files});
      }
    });
  }

  useEffect(() => {
    const handleResize = () => {
      const divRefHeight = divRef.current?.offsetHeight ?? 0;
      const newJacketSize = Math.min(
        window.innerWidth,
        window.innerHeight - divRefHeight
      );
      setJacketSize(newJacketSize);
    };

    document.body.classList.add("bg-gray-900");

    window.addEventListener("resize", handleResize);
    handleResize();
    return () => window.removeEventListener("resize", handleResize);
  }, []);

  return (
    <div className="flex flex-col h-screen overflow-hidden">
      <div className=" flex-1 flex justify-center items-center">
        <div style={{ width: jacketSize, height: jacketSize }} className="p-8">
          <div className="h-full bg-gray-800 flex justify-center items-center">
            <IoMusicalNotesSharp
              className="text-gray-500 -translate-x-[5%] hover:cursor-grab"
              size="75%"
              onClick={openDialog}
            />
          </div>
        </div>
      </div>
      <div ref={divRef}>
        <div className="flex justify-center text-center">
          <div className="break-all mx-8 mb-4 font-bold flex text-3xl text-gray-100 justify-center text-center">
            MONTERO (Call Me By Your Name)
          </div>
        </div>
        <div className="break-all mx-8 mb-8 flex text-xl text-gray-500 justify-center text-center">
          Lil Nas X
        </div>
        <div className="mx-8">
          <div className="group relative w-full py-2">
            <div className="rounded-full w-full h-1 bg-gray-500 absolute top-0 bottom-0 left-0 right-0 m-auto" />
            <div style={{width: `${progress[0]}%`}} className="rounded-full h-1 bg-gray-300 group-hover:bg-cyan-500 absolute top-0 bottom-0 left-0 my-auto" />
            <div style={{left: `${progress[0]}%`}} className="rounded-full w-4 h-4 bg-gray-300 opacity-0 group-hover:opacity-100 absolute top-0 bottom-0 my-auto -translate-x-[50%]" />
          </div>
          <div className="flex">
            <div className="text-gray-500 text-sm">
              {formatTime(progress[1])}
            </div>
            <div className="text-gray-500 text-sm ml-auto">
              {formatTime(progress[2])}
            </div>
          </div>
        </div>
        <div className="p-8 flex justify-center text-center">
          <button className="cursor-default text-4xl text-gray-300 hover:text-gray-50 hover:scale-105 active:text-gray-300 active:scale-100">
            <IoPlayBackSharp />
          </button>
          <button className="mx-16 text-5xl translate-x-1 cursor-default text-gray-300 hover:text-white hover:scale-105 active:text-gray-300 active:scale-100">
            <IoPlaySharp />
          </button>
          <button className="cursor-default text-4xl text-gray-300 hover:text-white hover:scale-105 active:text-gray-300 active:scale-100">
            <IoPlayForwardSharp />
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
