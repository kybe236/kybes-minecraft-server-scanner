package de.kybe.module.modules;

import de.kybe.module.ToggleableModule;
import de.kybe.settings.BooleanSetting;
import de.kybe.settings.NullSetting;
import de.kybe.settings.StringSetting;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.client.multiplayer.ServerList;

import java.sql.*;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

import static de.kybe.Constants.mc;

public class ScannerModule extends ToggleableModule {
  public static ScannerModule INSTANCE;

  private final StringSetting dburl = new StringSetting("Database URL", "jdbc:postgresql://localhost:5555/mc_scanner");
  private final StringSetting dbuser = new StringSetting("Database User", "mc_scanner");
  private final StringSetting dbpassword = new StringSetting("Database Password", "");
  private final StringSetting query = new StringSetting("Query", "SELECT ip FROM servers LIMIT 10");
  public final BooleanSetting clearServers = (BooleanSetting) new BooleanSetting("Clear Servers", true).onChange((aBoolean, aBoolean2) -> {
    if (aBoolean2) {
      ServerList serverList = new ServerList(mc);
      serverList.save();
      System.out.println("Cleared server list.");
      ScannerModule.INSTANCE.clearServers.setValue(false);
    }
  });

  public ScannerModule() {
    super("Scanner Accessor");

    NullSetting ndburl = new NullSetting("Database URL");
    ndburl.addSubSetting(dburl);
    NullSetting ndbuser = new NullSetting("Database User");
    ndbuser.addSubSetting(dbuser);
    NullSetting ndbpassword = new NullSetting("Database Password");
    ndbpassword.addSubSetting(dbpassword);

    addSetting(
      ndburl,
      ndbuser,
      ndbpassword,
      query,
      clearServers
    );

    INSTANCE = this;
  }

  @Override
  protected void onToggled(boolean toggled) {
    if (!toggled) return;

    String url = dburl.getValue();
    String user = dbuser.getValue();
    String password = dbpassword.getValue();
    String sql = query.getValue();

    ServerList serverList = new ServerList(mc);
    serverList.load();

    try {
      Class.forName("org.postgresql.Driver");
    } catch (Exception e) {
      e.printStackTrace();
      return;
    }
    try (Connection conn = DriverManager.getConnection(url, user, password);
         PreparedStatement stat = conn.prepareStatement(sql);
         ResultSet rs = stat.executeQuery()) {
      while (rs.next()) {
        String ip = rs.getString("ip");
        System.out.println("Found IP: " + ip);

        serverList.add(new ServerData(ip, ip, ServerData.Type.OTHER), false);
      }
    } catch (SQLException e) {
      e.printStackTrace();
      serverList.save();
    }
    serverList.save();
    this.setToggled(false);
  }


  @SuppressWarnings("unused")
  public List<String> getPlayers(String ip) {
    String sql = "SELECT pl.name " +
      "FROM servers " +
      "JOIN public.player_actions pa ON servers.id = pa.server_id " +
      "JOIN public.player_list pl ON pa.user_id = pl.id " +
      "WHERE ip = ?";

    Set<String> playersSet = new HashSet<>();

    try (Connection conn = DriverManager.getConnection(dburl.getValue(), dbuser.getValue(), dbpassword.getValue());
         PreparedStatement stat = conn.prepareStatement(sql)) {

      stat.setString(1, ip);

      try (ResultSet rs = stat.executeQuery()) {
        while (rs.next()) {
          playersSet.add(rs.getString("name"));
        }
      }
    } catch (SQLException e) {
      e.printStackTrace();
    }

    return new ArrayList<>(playersSet);
  }
}
