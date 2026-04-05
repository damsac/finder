import SwiftUI
import AVFoundation

@main
struct FinderApp: App {
    var body: some Scene {
        WindowGroup {
            FinderView()
        }
    }
}

struct FinderView: View {
    @StateObject private var camera = CameraModel()
    @State private var query: String = ""
    @State private var isSearching = false
    @State private var detection: Detection? = nil
    @State private var showResult = false

    var body: some View {
        ZStack {
            // Full screen camera preview
            CameraPreview(session: camera.session)
                .ignoresSafeArea()

            // Bounding box overlay when found
            if let det = detection, det.found {
                GeometryReader { geo in
                    let rect = CGRect(
                        x: det.xMin * geo.size.width,
                        y: det.yMin * geo.size.height,
                        width: (det.xMax - det.xMin) * geo.size.width,
                        height: (det.yMax - det.yMin) * geo.size.height
                    )
                    Rectangle()
                        .stroke(Color.green, lineWidth: 3)
                        .frame(width: rect.width, height: rect.height)
                        .position(x: rect.midX, y: rect.midY)

                    // Label above box
                    Text("\(query) (\(Int(det.confidence * 100))%)")
                        .font(.caption)
                        .fontWeight(.bold)
                        .foregroundColor(.white)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 4)
                        .background(Color.green.opacity(0.8))
                        .cornerRadius(6)
                        .position(x: rect.midX, y: rect.minY - 16)
                }
                .ignoresSafeArea()
            }

            // Bottom controls
            VStack {
                Spacer()

                if isSearching {
                    HStack(spacing: 8) {
                        ProgressView()
                            .tint(.white)
                        Text("Searching for \"\(query)\"...")
                            .foregroundColor(.white)
                            .font(.callout)
                    }
                    .padding()
                    .background(.ultraThinMaterial)
                    .cornerRadius(12)
                    .padding(.bottom, 8)
                }

                HStack(spacing: 12) {
                    TextField("What are you looking for?", text: $query)
                        .textFieldStyle(.roundedBorder)
                        .submitLabel(.search)
                        .onSubmit { startSearch() }
                        .disabled(isSearching)

                    Button(action: {
                        if isSearching { stopSearch() } else { startSearch() }
                    }) {
                        Image(systemName: isSearching ? "stop.fill" : "magnifyingglass")
                            .font(.title2)
                            .foregroundColor(.white)
                            .frame(width: 44, height: 44)
                            .background(isSearching ? Color.red : Color.blue)
                            .cornerRadius(22)
                    }
                    .disabled(query.isEmpty)
                }
                .padding()
                .background(.ultraThinMaterial)
            }
        }
        .onAppear { camera.start() }
        .onDisappear { camera.stop() }
        .accessibilityElement(children: .contain)
        .accessibilityIdentifier("finder_view")
    }

    private func startSearch() {
        guard !query.isEmpty else { return }
        isSearching = true
        detection = nil
        camera.startDetection(query: query) { result in
            DispatchQueue.main.async {
                self.detection = result
                if result.found {
                    self.isSearching = false
                    // Haptic feedback
                    let generator = UINotificationFeedbackGenerator()
                    generator.notificationOccurred(.success)
                }
            }
        }
    }

    private func stopSearch() {
        isSearching = false
        detection = nil
        camera.stopDetection()
    }
}

/// Simple detection result for the view
struct Detection {
    let found: Bool
    let confidence: Double
    let xMin: Double
    let yMin: Double
    let xMax: Double
    let yMax: Double
}

/// Camera model — manages AVCaptureSession and frame sampling
class CameraModel: NSObject, ObservableObject, AVCaptureVideoDataOutputSampleBufferDelegate {
    let session = AVCaptureSession()
    private let output = AVCaptureVideoDataOutput()
    private let queue = DispatchQueue(label: "camera.frame")
    private let detectQueue = DispatchQueue(label: "detection", qos: .userInitiated)

    private var isDetecting = false
    private var currentQuery = ""
    private var onDetection: ((Detection) -> Void)?
    private var lastSampleTime: Date = .distantPast
    // NOTE: AppCore integration commented out for initial build.
    // Uncomment when UniFFI bindings are generated.
    // private var appCore: AppCore?

    override init() {
        super.init()
        configure()
    }

    private func configure() {
        session.beginConfiguration()
        session.sessionPreset = .hd1280x720

        guard let device = AVCaptureDevice.default(.builtInWideAngleCamera, for: .video, position: .back),
              let input = try? AVCaptureDeviceInput(device: device) else { return }

        if session.canAddInput(input) { session.addInput(input) }

        output.videoSettings = [kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_32BGRA]
        output.setSampleBufferDelegate(self, queue: queue)
        output.alwaysDiscardsLateVideoFrames = true

        if session.canAddOutput(output) { session.addOutput(output) }

        session.commitConfiguration()
    }

    func start() {
        DispatchQueue.global(qos: .background).async {
            self.session.startRunning()
        }
    }

    func stop() {
        session.stopRunning()
    }

    func startDetection(query: String, onDetection: @escaping (Detection) -> Void) {
        self.currentQuery = query
        self.onDetection = onDetection
        self.isDetecting = true

        // Initialize AppCore if needed
        // let dataDir = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0].path
        // let apiKey = Bundle.main.infoDictionary?["PPQAPIKey"] as? String ?? ""
        // self.appCore = try? AppCore(apiKey: apiKey, dataDir: dataDir)
    }

    func stopDetection() {
        isDetecting = false
        onDetection = nil
    }

    // AVCaptureVideoDataOutputSampleBufferDelegate
    func captureOutput(_ output: AVCaptureOutput, didOutput sampleBuffer: CMSampleBuffer, from connection: AVCaptureConnection) {
        guard isDetecting else { return }

        // Sample at ~2fps
        let now = Date()
        guard now.timeIntervalSince(lastSampleTime) >= 0.5 else { return }
        lastSampleTime = now

        // Convert sample buffer to JPEG
        guard let jpegData = jpegFromSampleBuffer(sampleBuffer) else { return }

        // Send to detection on background queue
        detectQueue.async { [weak self] in
            guard let self = self, self.isDetecting else { return }

            // TODO: Replace with actual AppCore.detect() call when bindings are ready
            // if let core = self.appCore {
            //     do {
            //         let result = try core.detect(query: self.currentQuery, jpegBytes: Array(jpegData))
            //         let detection = Detection(
            //             found: result.found,
            //             confidence: result.confidence,
            //             xMin: result.bboxXMin,
            //             yMin: result.bboxYMin,
            //             xMax: result.bboxXMax,
            //             yMax: result.bboxYMax
            //         )
            //         self.onDetection?(detection)
            //     } catch {
            //         // Continue scanning on error
            //     }
            // }

            // Placeholder: simulate no detection
            let detection = Detection(found: false, confidence: 0, xMin: 0, yMin: 0, xMax: 0, yMax: 0)
            self.onDetection?(detection)
        }
    }

    private func jpegFromSampleBuffer(_ buffer: CMSampleBuffer) -> Data? {
        guard let pixelBuffer = CMSampleBufferGetImageBuffer(buffer) else { return nil }
        let ciImage = CIImage(cvPixelBuffer: pixelBuffer)
        let context = CIContext()
        guard let cgImage = context.createCGImage(ciImage, from: ciImage.extent) else { return nil }
        let uiImage = UIImage(cgImage: cgImage)
        return uiImage.jpegData(compressionQuality: 0.7)
    }
}

/// UIViewRepresentable for AVCaptureSession preview
struct CameraPreview: UIViewRepresentable {
    let session: AVCaptureSession

    func makeUIView(context: Context) -> UIView {
        let view = UIView(frame: .zero)
        let previewLayer = AVCaptureVideoPreviewLayer(session: session)
        previewLayer.videoGravity = .resizeAspectFill
        view.layer.addSublayer(previewLayer)

        // Store layer ref for updates
        view.tag = 42
        return view
    }

    func updateUIView(_ uiView: UIView, context: Context) {
        if let layer = uiView.layer.sublayers?.first as? AVCaptureVideoPreviewLayer {
            layer.frame = uiView.bounds
        }
    }
}
