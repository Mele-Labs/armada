// Report, from a right-click menu, to the annotation layer that carries it out. A window event,
// because the layer is a root of its own beside the app's and the app must not import it.

import { useEffect, useRef } from "react";

const REPORT = "armada:report";

/** Whether the annotation layer is mounted here, so a menu offers Report only where it can be carried out. */
export const canReport = (): boolean => window.armadaDev !== undefined || import.meta.env.DEV;

/** Begin a note on `element`, with the layer on or off. */
export function askToReport(element: Element): void {
  window.dispatchEvent(new CustomEvent<Element>(REPORT, { detail: element }));
}

export function useReportAsked(report: (element: Element) => void): void {
  const latest = useRef(report);
  latest.current = report;
  useEffect(() => {
    const asked = (event: Event): void => {
      if (event instanceof CustomEvent && event.detail instanceof Element) latest.current(event.detail);
    };
    window.addEventListener(REPORT, asked);
    return () => window.removeEventListener(REPORT, asked);
  }, []);
}

const REPORT_JOB = "armada:report-job";

/** Open the Job's own Report box, the one its flight controls' Report opens, from somewhere outside them. */
export function askToReportJob(session: string): void {
  window.dispatchEvent(new CustomEvent<string>(REPORT_JOB, { detail: session }));
}

export function useJobReportAsked(session: string, open: () => void): void {
  const latest = useRef(open);
  latest.current = open;
  useEffect(() => {
    const asked = (event: Event): void => {
      if (event instanceof CustomEvent && event.detail === session) latest.current();
    };
    window.addEventListener(REPORT_JOB, asked);
    return () => window.removeEventListener(REPORT_JOB, asked);
  }, [session]);
}
