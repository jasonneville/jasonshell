export function shouldApplySystemSearchResponse(
  requestQuery: string,
  requestSequence: number,
  currentQuery: string,
  currentSequence: number
) {
  return requestSequence === currentSequence && requestQuery === currentQuery.trim();
}

export function shouldRetryIndexedSearch(resultsLength: number, refreshAttempt: number) {
  return resultsLength === 0 && refreshAttempt < 2;
}

export function shouldRefreshSystemSearchAfterIndexUpdate(
  searchOpen: boolean,
  currentQuery: string
) {
  return searchOpen && currentQuery.trim().length >= 2;
}
