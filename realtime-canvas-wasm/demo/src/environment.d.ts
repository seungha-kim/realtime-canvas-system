declare global {
  type Process = {
    env: any;
  };
  const process: Process;
}

// If this file has no import/export statements (i.e. is a script)
// convert it into a module by adding an empty export statement.
export {};
