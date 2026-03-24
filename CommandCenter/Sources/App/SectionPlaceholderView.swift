import SwiftUI

struct SectionPlaceholderView: View {
    let item: SidebarItem

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Label(item.title, systemImage: item.icon)
                .font(.largeTitle.weight(.semibold))

            Text(item.summary)
                .font(.title3)
                .foregroundStyle(.secondary)

            VStack(alignment: .leading, spacing: 12) {
                Text("Phase 3 foundation")
                    .font(.headline)
                Text(
                    "This native SwiftUI surface replaces the legacy embedded dashboard "
                        + "with first-class macOS views tuned for Tahoe and Liquid Glass."
                )
                .foregroundStyle(.secondary)
            }
            .padding(24)
            .frame(maxWidth: 560, alignment: .leading)
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: 24, style: .continuous))

            Spacer()
        }
        .padding(32)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
