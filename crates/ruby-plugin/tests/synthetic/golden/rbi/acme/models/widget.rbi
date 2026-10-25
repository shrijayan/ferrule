# typed: strong

module Acme
  module Models
    class Widget < Acme::Internal::Type::BaseModel
      OrHash = T.type_alias { T.any(Acme::Widget, Acme::Internal::AnyHash) }

      sig { returns(String) }
      attr_accessor :id

      sig { returns(Time) }
      attr_accessor :created_at

      sig { returns(String) }
      attr_accessor :name

      # Where the widget is in its life cycle.
      sig { returns(Acme::Widget::Status::TaggedSymbol) }
      attr_accessor :status

      sig { returns(T.nilable(String)) }
      attr_reader :homepage_url

      sig { params(homepage_url: String).void }
      attr_writer :homepage_url

      # Labels by language.
      sig { returns(T.nilable(T::Hash[Symbol, Acme::LabelsItem])) }
      attr_reader :labels

      sig { params(labels: T::Hash[Symbol, Acme::LabelsItem::OrHash]).void }
      attr_writer :labels

      sig { returns(T.nilable(T::Array[Acme::Widget::Part])) }
      attr_reader :parts

      sig { params(parts: T::Array[Acme::Widget::Part::OrHash]).void }
      attr_writer :parts

      sig { returns(T.nilable(String)) }
      attr_accessor :sku

      sig { returns(T.nilable(Float)) }
      attr_reader :weight_kg

      sig { params(weight_kg: Float).void }
      attr_writer :weight_kg

      # A widget in the catalogue.
      sig { params(id: String, created_at: Time, name: String, status: Acme::Widget::Status::OrSymbol, homepage_url: String, labels: T::Hash[Symbol, Acme::LabelsItem::OrHash], parts: T::Array[Acme::Widget::Part::OrHash], sku: T.nilable(String), weight_kg: Float).returns(T.attached_class) }
      def self.new(
        id:,
        created_at:,
        name:,
        # Where the widget is in its life cycle.
        status:,
        homepage_url: nil,
        # Labels by language.
        labels: nil,
        parts: nil,
        sku: nil,
        weight_kg: nil
      )
      end

      sig { override.returns({ id: String, created_at: Time, name: String, status: Acme::Widget::Status::TaggedSymbol, homepage_url: String, labels: T::Hash[Symbol, Acme::LabelsItem], parts: T::Array[Acme::Widget::Part], sku: T.nilable(String), weight_kg: Float }) }
      def to_hash
      end

      # Where the widget is in its life cycle.
      module Status
        extend Acme::Internal::Type::Enum

        TaggedSymbol = T.type_alias { T.all(Symbol, Acme::Widget::Status) }
        OrSymbol = T.type_alias { T.any(Symbol, String) }

        DRAFT = T.let(:draft, Acme::Widget::Status::TaggedSymbol)
        ACTIVE = T.let(:active, Acme::Widget::Status::TaggedSymbol)
        RETIRED = T.let(:retired, Acme::Widget::Status::TaggedSymbol)

        sig { override.returns(T::Array[Acme::Widget::Status::TaggedSymbol]) }
        def self.values
        end
      end

      class Part < Acme::Internal::Type::BaseModel
        OrHash = T.type_alias { T.any(Acme::Widget::Part, Acme::Internal::AnyHash) }

        sig { returns(T.nilable(String)) }
        attr_reader :label

        sig { params(label: String).void }
        attr_writer :label

        sig { returns(T.nilable(Integer)) }
        attr_reader :part_number

        sig { params(part_number: Integer).void }
        attr_writer :part_number

        sig { params(label: String, part_number: Integer).returns(T.attached_class) }
        def self.new(label: nil, part_number: nil)
        end

        sig { override.returns({ label: String, part_number: Integer }) }
        def to_hash
        end
      end
    end
  end
end
