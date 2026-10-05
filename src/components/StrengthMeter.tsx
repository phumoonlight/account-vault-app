import { passwordStrength } from "../strength";

export function StrengthMeter({ password }: { password: string }) {
  const strength = passwordStrength(password);
  if (!strength) return null;

  return (
    <div className={`strength strength-${strength.score}`} aria-live="polite">
      <div className="strength-bar" aria-hidden>
        {[0, 1, 2, 3].map((i) => (
          <span key={i} className={i < Math.max(strength.score, 1) ? "on" : ""} />
        ))}
      </div>
      <span className="strength-text">
        <strong>{strength.label}</strong>
        {strength.tip && <> · {strength.tip}</>}
      </span>
    </div>
  );
}
