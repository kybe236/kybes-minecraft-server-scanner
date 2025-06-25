package de.kybe.command.commands;

import de.kybe.command.Command;
import de.kybe.utils.ChatUtils;
import net.minecraft.network.protocol.game.ServerboundMovePlayerPacket;
import net.minecraft.network.protocol.game.ServerboundMoveVehiclePacket;

import static de.kybe.Constants.mc;

public class VClipCommand extends Command {
  public VClipCommand() {
    super("vclip", "vc");
  }

  @Override
  public void execute(String[] args) {
    if (mc.player == null) return;
    if (args.length < 1) {
      ChatUtils.print("Usage: /vclip <distance>");
      return;
    }

    try {
      double distance = Double.parseDouble(args[0]);

      int packetsRequired = (int) Math.ceil(Math.abs(distance / 10));
      if (packetsRequired > 20) {
        packetsRequired = 1;
      }

      if (mc.player.isVehicle()) {
        for (int packetNumber = 0; packetNumber < (packetsRequired - 1); packetNumber++) {
          mc.player.connection.send(ServerboundMoveVehiclePacket.fromEntity(mc.player.getVehicle()));
        }
        mc.player.getVehicle().setPos(mc.player.getVehicle().getX(), mc.player.getVehicle().getY() + distance, mc.player.getVehicle().getZ());
        mc.player.connection.send(ServerboundMoveVehiclePacket.fromEntity(mc.player.getVehicle()));
      } else {
        for (int packetNumber = 0; packetNumber < (packetsRequired - 1); packetNumber++) {
          mc.player.connection.send(new ServerboundMovePlayerPacket.StatusOnly(true, mc.player.horizontalCollision));
        }
        mc.player.connection.send(new ServerboundMovePlayerPacket.Pos(mc.player.getX(), mc.player.getY() + distance, mc.player.getZ(), true, mc.player.horizontalCollision));
        mc.player.setPos(mc.player.getX(), mc.player.getY() + distance, mc.player.getZ());
      }
    } catch (NumberFormatException e) {
      ChatUtils.print("Invalid distance: " + args[0]);
    }
  }
}
