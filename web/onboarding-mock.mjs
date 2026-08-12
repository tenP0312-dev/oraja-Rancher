export const ONBOARDING_MOCK_STAGES = Object.freeze([
  "account",
  "profile",
  "connection",
  "ready"
]);

export function createOnboardingMockState() {
  return {
    stage: "account",
    accountMode: "new",
    accountLinked: false,
    accountId: null,
    accountName: null,
    profileId: "PLAYER",
    profileConfigured: false,
    connectionTesting: false,
    connectionPassed: false,
    launchUnavailable: false
  };
}

export function validateOnboardingMockAccount(input) {
  const mode = input?.mode === "existing" ? "existing" : "new";
  const password = String(input?.password || "");
  if (mode === "new") {
    const displayName = String(input?.displayName || "").trim();
    if (!displayName) return {valid: false, error: "displayNameRequired"};
    if (displayName.length > 64) return {valid: false, error: "displayNameTooLong"};
    if (password.length < 6) return {valid: false, error: "passwordTooShort"};
    if (password !== String(input?.passwordConfirmation || "")) {
      return {valid: false, error: "passwordMismatch"};
    }
    if (!input?.termsAccepted) return {valid: false, error: "termsRequired"};
    return {valid: true, accountId: "190000", accountName: displayName};
  }

  const playerId = String(input?.playerId || "").trim();
  if (!/^\d+$/.test(playerId) || Number(playerId) < 190000 || Number(playerId) > 2147483647) {
    return {valid: false, error: "playerIdInvalid"};
  }
  if (!password) return {valid: false, error: "passwordRequired"};
  return {valid: true, accountId: playerId, accountName: "BMSIR_MOCK"};
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
    case "SELECT_ACCOUNT_MODE":
      if (state.stage !== "account") return state;
      return {
        ...state,
        accountMode: event.mode === "existing" ? "existing" : "new",
        launchUnavailable: false
      };
    case "COMPLETE_ACCOUNT":
      if (state.stage !== "account" || !event.accountId || !event.accountName) return state;
      return {
        ...state,
        stage: "profile",
        accountLinked: true,
        accountId: String(event.accountId),
        accountName: String(event.accountName)
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
