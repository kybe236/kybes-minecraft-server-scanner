package de.kybe.module.modules;

import de.kybe.event.KybeEvents;
import de.kybe.event.events.EventTick;
import de.kybe.module.ToggleableModule;
import de.kybe.settings.NumberSetting;
import net.minecraft.world.phys.Vec3;

import static de.kybe.Constants.mc;

public class SpeedModule extends ToggleableModule {
  public NumberSetting<Double> multiplier = new NumberSetting<>("Multiplier", 1.0);

  public SpeedModule() {
    super("Speed");

    this.addSetting(multiplier);
  }

  @KybeEvents
  @SuppressWarnings("unused")
  public void onTick(EventTick event) {
    if (!this.isToggled()) return;
    if (mc.player == null) return;

    double speedMult = multiplier.getValue();
    if (speedMult <= 1.0) return;

    boolean forward = mc.options.keyUp.isDown();
    boolean back = mc.options.keyDown.isDown();
    boolean left = mc.options.keyLeft.isDown();
    boolean right = mc.options.keyRight.isDown();

    Vec3 lookVec = mc.player.getLookAngle();
    Vec3 motion = new Vec3(0, mc.player.getDeltaMovement().y, 0); // preserve vertical motion

    Vec3 forwardVec = new Vec3(lookVec.x, 0, lookVec.z).normalize();
    Vec3 rightVec = new Vec3(-lookVec.z, 0, lookVec.x).normalize();

    if (forward) motion = motion.add(forwardVec);
    if (back) motion = motion.subtract(forwardVec);
    if (right) motion = motion.add(rightVec);
    if (left) motion = motion.subtract(rightVec);

    if (motion.lengthSqr() > 0) {
      motion = motion.normalize().scale(speedMult);
      motion = new Vec3(motion.x, mc.player.getDeltaMovement().y, motion.z);
    }

    mc.player.setDeltaMovement(motion);
  }
}
