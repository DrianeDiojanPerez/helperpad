import com.topaz.sigplus.SigPlus;
import java.io.File;
import java.nio.file.Files;
import java.nio.file.Path;
import javax.imageio.ImageIO;

public class Check {
    public static void main(String[] args) throws Exception {
        String hex = Files.readString(Path.of(args[0])).replaceFirst("^\\s*SigString:", "").replaceAll("\\s", "");
        int mode = Integer.parseInt(args[1]);

        SigPlus sig = new SigPlus();
        sig.setSigCompressionMode(mode);
        sig.setSigString(hex);

        int strokes = sig.getNumberOfStrokes();
        StringBuilder points = new StringBuilder();
        for (int i = 0; i < strokes; i++) {
            points.append(i == 0 ? "" : ",").append(sig.getNumPointsForStroke(i));
        }
        System.out.println("strokes: " + strokes);
        System.out.println("points per stroke: " + points);
        System.out.println("total points: " + sig.numberOfTabletPoints());
        System.out.println("round trip identical: " + sig.getSigString().equals(hex.toUpperCase()));

        if (args.length > 2) {
            sig.setImageXSize(1000);
            sig.setImageYSize(300);
            sig.setImagePenWidth(6);
            ImageIO.write(sig.sigImage(), "png", new File(args[2]));
            System.out.println("drawn by Topaz SigPlus: " + args[2]);
        }
        System.exit(0);
    }
}
