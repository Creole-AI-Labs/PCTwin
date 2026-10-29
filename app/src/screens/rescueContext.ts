/**
 * Rescue a laptop (2.6): the move's pages on this laptop are the new laptop's, with a drive as the
 * old laptop. While a rescue runs, MoveHost provides the drive's name here, and the pages whose
 * words speak of "your old laptop" or of a person sending say what fits a drive instead.
 */
import { createContext, useContext } from "react";

export const RescueDrive = createContext<string | null>(null);

/** The rescued drive's name, or null when this is an ordinary move. */
export function useRescueDrive(): string | null {
  return useContext(RescueDrive);
}
