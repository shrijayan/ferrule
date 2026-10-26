export default {
  extends: ['@commitlint/config-conventional'],
  // Dependabot writes "chore(deps): Bump ..." (dependabot.yml sets the prefix and scope); its
  // capital B breaks subject-case, and its messages are not ours to reword.
  ignores: [(message) => /^(chore|ci)\(deps(-dev)?\): Bump /.test(message)],
};
