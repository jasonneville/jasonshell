export type TopBarIdentityState = {
  placesLabel: string;
  programsLabel: string;
  indexLabel: string;
  workspaceLabel: string;
};

export function topBarIdentityState(
  pinnedPlaceCount: number,
  programCount: number,
  searchStatus: string
): TopBarIdentityState {
  return {
    placesLabel: pinnedPlaceCount === 1 ? '1 place' : `${pinnedPlaceCount} places`,
    programsLabel: programCount === 1 ? '1 app' : `${programCount} apps`,
    indexLabel: searchStatus.toLocaleLowerCase().includes('searching')
      ? 'Indexing'
      : 'Index ready',
    workspaceLabel: 'Workspace later'
  };
}
