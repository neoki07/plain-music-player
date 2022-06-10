import { useEffect, useRef, useState } from "react";
import {
  IoPlaySharp,
  IoPlayForwardSharp,
  IoPlayBackSharp,
  IoMusicalNotesSharp,
  IoPauseSharp
} from "react-icons/io5";
import { open } from "@tauri-apps/api/dialog";
import { invoke } from '@tauri-apps/api/tauri';

type Progress = [number, number, number];

function formatTime(time: number) {
  const min = Math.floor(time / 60);
  const sec = time % 60;
  return `${min}:${sec.toString().padStart(2, "0")}`;
}

function App() {
  const divExcludingCoverRef = useRef<HTMLDivElement>(null);
  const divProgressBarRef = useRef<HTMLDivElement>(null);
  const [coverSize, setCoverSize] = useState(0);
  const [progress, setProgress] = useState<Progress>([0, 0, 0]);
  const progressRef = useRef<Progress>(progress);
  const [isDraggingProgressBar, setIsDraggingProgressBar] = useState(false);
  const isDraggingProgressBarRef = useRef(isDraggingProgressBar);
  const [isPaused, setIsPaused] = useState(true);
  const isRightAfterSeekRef = useRef(false);

  const requestIdRef = useRef(0);
  const animate = () => {
    invoke("is_paused").then((isPaused) => setIsPaused(isPaused as boolean));
    if (!isDraggingProgressBarRef.current && !isRightAfterSeekRef.current) {
      invoke("get_progress").then((progress) => {
        setProgress(progress as Progress);
      });
    }
    requestIdRef.current = requestAnimationFrame(animate)
  };
  useEffect(() => {
    requestIdRef.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(requestIdRef.current);
  }, [])

  useEffect(() => {
    console.log("get progress:", progress);
  }, [progress])

  function openDialog() {
    open().then((files) => {
      if (files && typeof files == "string") {
        invoke("play", {path: files});
      }
    });
  }

  useEffect(() => {
    const handleResize = () => {
      const divExcludingCoverHeight = divExcludingCoverRef.current?.offsetHeight ?? 0;
      const newCoverSize = Math.min(
        window.innerWidth,
        window.innerHeight - divExcludingCoverHeight
      );
      setCoverSize(newCoverSize);
    };

    const handleMouseMove = (event: MouseEvent) => {
      event.preventDefault();

      if (divProgressBarRef.current && isDraggingProgressBarRef.current) {
        const mouseX = event.clientX;
        const {x: progressBarX, width: progressBarWidth} = divProgressBarRef.current.getBoundingClientRect();
        const duration = progressRef.current[2];
        const time = mouseX <= progressBarX ? 0 : mouseX >= progressBarX + progressBarWidth ? duration : (mouseX - progressBarX) / progressBarWidth * duration;
        console.log("time:", time);
        setProgress([time / duration * 100, Math.floor(time), duration]);
      }
    };

    const handleMouseUp = (event: MouseEvent) => {
      setIsDraggingProgressBar(false);

      if (divProgressBarRef.current && isDraggingProgressBarRef.current) {
        const mouseX = event.clientX;
        const {x: progressBarX, width: progressBarWidth} = divProgressBarRef.current.getBoundingClientRect();
        const duration = progressRef.current[2];
        const time = mouseX <= progressBarX ? 0 : mouseX >= progressBarX + progressBarWidth ? duration : (mouseX - progressBarX) / progressBarWidth * duration;
        console.log("time:", time);
        setProgress([time / duration * 100, Math.floor(time), duration]);
        invoke("seek_to", { time: Math.floor(time) })
        isRightAfterSeekRef.current = true;

        setTimeout(() => {
          isRightAfterSeekRef.current = false;
        }, 50);
      }
    };

    document.body.classList.add("bg-gray-900");

    window.addEventListener("resize", handleResize);
    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    handleResize();
    return () => {
      window.removeEventListener("resize", handleResize);
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    }
  }, []);

  useEffect(() => {
    isDraggingProgressBarRef.current = isDraggingProgressBar
  }, [isDraggingProgressBar])

  useEffect(() => {
    progressRef.current = progress
  }, [progress])

  return (
    <div className="flex flex-col h-screen overflow-hidden">
      <div className=" flex-1 flex justify-center items-center">
        <div style={{ width: coverSize, height: coverSize }} className="p-8">
          <div className="h-full bg-gray-800 flex justify-center items-center">
            <IoMusicalNotesSharp
              className="text-gray-500 -translate-x-[5%] hover:cursor-grab"
              size="75%"
              onClick={openDialog}
            />
          </div>
        </div>
      </div>
      <div ref={divExcludingCoverRef}>
        <div className="flex justify-center items-center">
          <div className="break-all mx-8 font-bold flex text-3xl text-gray-100 justify-center items-center">
            MONTERO (Call Me By Your Name)
          </div>
        </div>
        <div className="break-all mx-8 mb-4 flex text-lg text-gray-500 justify-center items-center">
          Lil Nas X
        </div>
        <div className="mx-8">
          <div className="group relative w-full ">
            <div
                ref={divProgressBarRef}
                className="py-2"
                onMouseDown={(event) => {
                  setIsDraggingProgressBar(true);

                  const mouseX = event.clientX;
                  const {x: progressBarX, width: progressBarWidth} = event.currentTarget.getBoundingClientRect();
                  const duration = progress[2];
                  const time = mouseX <= progressBarX ? 0 : mouseX >= progressBarX + progressBarWidth ? duration : (mouseX - progressBarX) / progressBarWidth * (duration + 1);
                  console.log("time:", time);
                  setProgress([time / duration * 100, Math.floor(time), duration]);
                }}
            >
              <div className="rounded-full w-full h-1 bg-gray-500 absolute top-0 bottom-0 left-0 right-0 m-auto" />
            </div>
            <div style={{width: `${progress[0]}%`}} className="rounded-full h-1 bg-gray-300 group-hover:bg-cyan-500 absolute top-0 bottom-0 left-0 my-auto pointer-events-none" />
            <div style={{left: `${progress[0]}%`}} className="rounded-full w-4 h-4 bg-gray-300 opacity-0 group-hover:opacity-100 absolute top-0 bottom-0 my-auto -translate-x-[50%] pointer-events-none" />
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
        <div className="p-8 pt-4 flex justify-center items-center">
          <button className="cursor-default text-4xl text-gray-300 hover:text-gray-50 hover:scale-105 active:text-gray-300 active:scale-100">
            <IoPlayBackSharp />
          </button>
          <button
              className="mx-16 text-5xl translate-x-1 cursor-default text-gray-300 hover:text-white hover:scale-105 active:text-gray-300 active:scale-100"
              onClick={() => {
                if (isPaused) {
                  invoke("resume")
                } else {
                  invoke("pause")
                }
              }}
          >
            {isPaused ? <IoPlaySharp /> : <IoPauseSharp />}
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
