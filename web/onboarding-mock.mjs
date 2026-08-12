export const ONBOARDING_MOCK_STAGES = Object.freeze([
  "account",
  "profile",
  "connection",
  "ready"
]);

export function createOnboardingMockState() {
  return {
    stage: "account",
    linkMethod: null,
    linkPending: false,
    accountLinked: false,
    profileId: "PLAYER",
    profileConfigured: false,
    connectionTesting: false,
    connectionPassed: false,
    launchUnavailable: false
  };
}

export function onboardingMockStepState(state, stage) {
  const current = ONBOARDING_MOCK_STAGES.indexOf(state.stage);
  const target = ONBOARDING_MOCK_STAGES.indexOf(stage);
  if (target < current || (stage === "ready" && state.connectionPassed)) return "complete";
  return target === current ? "current" : "pending";
}

export function completedOnboardingMockSteps(state) {
  return [
    state.accountLinked,
    state.profileConfigured,
    state.connectionPassed,
    state.stage === "ready" && state.connectionPassed
  ].filter(Boolean).length;
}

export function reduceOnboardingMock(state, event) {
  switch (event.type) {
    case "START_ACCOUNT_LINK":
      if (state.stage !== "account") return state;
      return {
        ...state,
        linkMethod: event.method === "new" ? "new" : "existing",
        linkPending: true,
        launchUnavailable: false
      };
    case "COMPLETE_ACCOUNT_LINK":
      if (state.stage !== "account" || !state.linkPending) return state;
      return {
        ...state,
        stage: "profile",
        linkPending: false,
        accountLinked: true
      };
    case "CONFIGURE_PROFILE":
      if (state.stage !== "profile" || !state.accountLinked) return state;
      return {
        ...state,
        stage: "connection",
        profileId: String(event.profileId || "PLAYER"),
        profileConfigured: true
      };
    case "START_CONNECTION_TEST":
      if (state.stage !== "connection" || !state.profileConfigured) return state;
      return {...state, connectionTesting: true, launchUnavailable: false};
    case "COMPLETE_CONNECTION_TEST":
      if (state.stage !== "connection" || !state.connectionTesting) return state;
      return {
        ...state,
        stage: "ready",
        connectionTesting: false,
        connectionPassed: true
      };
    case "MARK_LAUNCH_UNAVAILABLE":
      if (state.stage !== "ready") return state;
      return {...state, launchUnavailable: true};
    case "RESET":
      return createOnboardingMockState();
    default:
      return state;
  }
}
